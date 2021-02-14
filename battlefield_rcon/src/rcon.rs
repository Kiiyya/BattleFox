use core::panic;
use std::{collections::HashMap, convert::TryInto, io::ErrorKind, str::FromStr, sync::Arc};

// use crate::error::{Error, Result};
use ascii::{AsciiString, FromAsciiError, IntoAsciiString};
use packet::{Packet, PacketDeserializeResult, PacketOrigin};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
};
use tokio::{
    net::tcp::OwnedReadHalf,
    sync::{mpsc, oneshot},
};
use tokio::{net::TcpStream, task::JoinHandle};

pub(crate) mod packet;

#[derive(Debug, Clone)]
pub enum RconError {
    /// Prominently when ip:port are wrong, etc.
    /// Arc because `std::io::Error` doesn't implement Clone...
    Io(Arc<std::io::Error>),

    /// Bad rcon password.
    /// Some day when automatic reconnecting will be added,
    /// This could be thrown on queries too.
    WrongPassword,

    /// An already established connection has been unexpectedly closed.
    /// Could be due to server shutdown I guess?
    ConnectionClosed,

    /// Malformed packets, bad sequence ids, etc...
    ProtocolError,

    /// Some string passed into this api could not be converted to Ascii.
    /// E.g. contains utf8 characters which don't exist in Ascii.
    NotAscii,

    /// Some rcon query/command timed out.
    TimedOut,

    /// When Rcon says this command does not exist.
    UnknownCommand,
    /// When Rcon says that something's wrong with the arguments to the command.
    InvalidArguments,
    /// When *we* don't know what the fuck rcon just responded to us. More like unknown query response.
    UnknownResponse,

    /// Some rare or very weird error.
    Other(String),
}

impl RconError {
    pub fn other(str: impl Into<String>) -> Self {
        RconError::Other(str.into())
    }
}

impl From<std::io::Error> for RconError {
    fn from(e: std::io::Error) -> Self {
        RconError::Io(Arc::new(e))
    }
}

impl<T> From<FromAsciiError<T>> for RconError {
    fn from(_: FromAsciiError<T>) -> Self {
        RconError::NotAscii
    }
}

pub type RconResult<T> = Result<T, RconError>;

#[derive(Debug)]
pub struct RconConnectionInfo {
    pub ip: String,
    pub port: u16,
    pub password: AsciiString,
}

impl Into<RconConnectionInfo> for (String, u16, AsciiString) {
    fn into(self) -> RconConnectionInfo {
        RconConnectionInfo {
            ip: self.0,
            port: self.1,
            password: self.2,
        }
    }
}

impl Into<RconConnectionInfo> for (&str, u16, &str) {
    fn into(self) -> RconConnectionInfo {
        RconConnectionInfo {
            ip: self.0.into(),
            port: self.1.into(),
            password: AsciiString::from_str(self.2).expect("Password is not ASCII."),
        }
    }
}

#[derive(Debug)]
pub struct RconClient {
    /// So that we can drop the rconclient later.
    mainloop: Option<std::thread::JoinHandle<()>>,
    /// In order to drop drop the rconclient, we need to tell the mainloop to stop.
    mainloop_shutdown: mpsc::UnboundedSender<()>,

    /// Sending a query to this will be handled by the mainloop.
    queries: mpsc::UnboundedSender<Query>,

    nonresponse_rx: Option<mpsc::UnboundedReceiver<RconResult<Packet>>>,

    /// ip, port, password.
    _connection_info: RconConnectionInfo,
}

#[derive(Debug)]
struct Query(
    Vec<AsciiString>,
    oneshot::Sender<RconResult<Vec<AsciiString>>>,
);

pub trait RconEventPacketHandler {
    fn on_packet(&self, packet: Packet);
}

/// Just used internally to do a remote procedure call.
impl RconClient {
    pub async fn connect(conn: impl Into<RconConnectionInfo>) -> RconResult<Self> {
        let conn: RconConnectionInfo = conn.into();
        let tcp = TcpStream::connect((conn.ip.clone(), conn.port)).await?;

        let (query_tx, query_rx) = mpsc::unbounded_channel::<Query>();
        let (shutdown_tx, shutdown_rx) = mpsc::unbounded_channel::<()>();
        let (nonresponses_tx, nonresponses_rx) = mpsc::unbounded_channel::<RconResult<Packet>>();

        let shutdown_rx = shutdown_rx;

        let tokio = tokio::runtime::Handle::current();
        let mainloop = std::thread::spawn(move || {
            tokio.spawn(RconClient::mainloop(
                query_rx,
                nonresponses_tx,
                tcp,
                shutdown_rx,
            ));
        });

        let myself = RconClient {
            mainloop: Some(mainloop),
            queries: query_tx,
            nonresponse_rx: Some(nonresponses_rx),
            mainloop_shutdown: shutdown_tx,

            _connection_info: RconConnectionInfo {
                password: conn.password.clone(),
                ..conn
            },
        };

        // at this point we should have a fully functional async way to query.
        // so we just login and set stuff up, and done!

        // TODO: use salted passwords eventually.
        myself
            .command(
                &veca!["login.plainText", conn.password],
                ok_eof::<RconError>,
                |err| match err {
                    "InvalidPassword" => Some(RconError::WrongPassword),
                    "PasswordNotSet" => Some(RconError::other("There is no password at all!")),
                    _ => None,
                },
            )
            .await?;

        Ok(myself)
    }

    pub fn take_nonresponse_rx(&mut self) -> Option<mpsc::UnboundedReceiver<RconResult<Packet>>> {
        self.nonresponse_rx.take()
    }

    /// tx stuff replies:
    /// - `Ok(Some(packet))` when normal, think of it as a stream.
    /// - `Ok(None)` stream ended gracefully (e.g. when shutdown signal sent).
    /// - `Err(e)` when rcon error.
    async fn tcp_read_loop(
        mut tcp: OwnedReadHalf,
        tx_responses: mpsc::UnboundedSender<RconResult<Packet>>,
        tx_nonresponses: mpsc::UnboundedSender<RconResult<Packet>>,
        mut shutdown_rx: mpsc::Receiver<()>,
    ) {
        let mut buf = vec![0_u8; 12]; // header size. We'll grow the buffer as necessary later.
        let mut ret: RconResult<()> = Ok(());
        let mut tx_responses_closed = false;
        let mut tx_nonresponses_closed = false;

        'outer: loop {
            if tx_responses_closed && tx_nonresponses_closed {
                // once both our output streams are closed, our job here is done.
                break;
            }
            // let start = Instant::now();
            tokio::select! {
                // read 12 byte header
                tcpread = tcp.read_exact(&mut buf[0..12]) => {
                    // make sure the read was successful
                    match tcpread {
                        Ok(n) if n == 0 => {
                            // ret = Err(RconError::ConnectionClosed);
                            ret = Ok(());
                            break 'outer;
                        },
                        Ok(n) => {
                            assert_eq!(n, 12);
                        },
                        Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                            ret = Err(RconError::ConnectionClosed);
                            // ret = Ok(());
                            break 'outer;
                        },
                        Err(e) => {
                            panic!("unexpected io error in tcp reader: {:?}", e);
                        }
                    };

                    let total_len = Packet::read_total_len(buf[0..12].try_into().unwrap());
                    if buf.len() < total_len {
                        buf.resize(total_len, 0);
                    }
                    // get rest of the packet, but also handle End potentially.
                    'inner: loop {
                        tokio::select! {
                            tcpread = tcp.read_exact(&mut buf[12..total_len]) => {
                                // make sure the read was successful
                                match tcpread {
                                    Ok(n) if n == 0 => {
                                        ret = Ok(());
                                        // ret = Err(RconError::ConnectionClosed);
                                        break 'outer;
                                    },
                                    Ok(n) => {
                                        assert_eq!(n, total_len - 12);
                                    },
                                    Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                                        ret = Err(RconError::ConnectionClosed);
                                        break 'outer;
                                    },
                                    Err(e) => {
                                        panic!("unexpected io error in tcp reader: {:?}", e);
                                    }
                                };

                                let packet = match Packet::deserialize(&buf[0..total_len]) {
                                    PacketDeserializeResult::Ok {packet, consumed_bytes} => {
                                        if consumed_bytes != total_len {
                                            ret = Err(RconError::ProtocolError);
                                            break 'outer;
                                        }
                                        packet
                                    },
                                    _ => {
                                        ret = Err(RconError::ProtocolError);
                                        break 'outer;
                                    }
                                };

                                // println!("In:  {}", packet);
                                if packet.is_response && !tx_responses_closed {
                                    if let Err(_e) = tx_responses.send(Ok(packet)) {
                                        // Receiver closed stream, means we're done here.
                                        tx_responses_closed = true;
                                    }
                                    break 'inner; // break inner loop => read next header for next packet.
                                } else if !packet.is_response && !tx_nonresponses_closed {
                                    // this will give it to Bf4Client,
                                    // which will read it and convert strings to types and then call its events_caller but then with a Bf4Event.
                                    if let Err(_e) = tx_nonresponses.send(Ok(packet)) {
                                        // Receiver closed stream, means we're done here.
                                        tx_nonresponses_closed = true;
                                    }
                                    break 'inner;
                                } else {
                                    panic!("This is never supposed to happen")
                                }
                            },
                            _ = shutdown_rx.recv() => {
                                println!("warn [Rcon tcp_read_loop] received shutdown signal, but had a packet partially read.");
                                break 'outer;
                            }
                        }
                    }
                },
                _ = shutdown_rx.recv() => break 'outer,
            }
        }

        match ret {
            Ok(()) => {
                // don't need to send anything here, since when all senders get dropped (and we have the only senders),
                // the stream gets closed automatically.

                // if !tx_nonresponses_closed {
                //     let _ = tx_nonresponses.send(Ok(None));
                // }
                // if !tx_responses_closed {
                //     let _ = tx_responses.send(Ok(None));
                // }
            }
            Err(e) => {
                if !tx_nonresponses_closed {
                    // if sending fails, that simply means it's been closed already
                    let _ = tx_nonresponses.send(Err(e.clone()));
                }
                if !tx_nonresponses_closed {
                    // if sending fails, that simply means it's been closed already
                    let _ = tx_responses.send(Err(e));
                }
            }
        }

        // println!("tcp_read_loop ended");
        // we drop the TCP half here.
        // we drop shutdown_rx here.
    }

    async fn mainloop(
        mut query_rx: mpsc::UnboundedReceiver<Query>,
        tx_nonresponses: mpsc::UnboundedSender<RconResult<Packet>>,
        tcp: TcpStream,
        mut shutdown: mpsc::UnboundedReceiver<()>,
    ) {
        // no need for mutexes locking the sequence numbers and `waiting`, since we're using message passing, and this is thread-local.
        struct Waiting {
            replier: oneshot::Sender<RconResult<Vec<AsciiString>>>,
            sent: std::time::Instant,
        }

        let mut sequence: u32 = 0;
        let mut waiting: HashMap<u32, Waiting> = HashMap::new();
        // let mut rcon_response_times = VecDeque::new();
        let (tcp_read, mut tcp_write) = tcp.into_split();

        struct Worker<T> {
            handle: JoinHandle<()>,
            x: T,
            shutdown: mpsc::Sender<()>,
        }
        let mut tcp_in = {
            let (tx_responses, rx_responses) = mpsc::unbounded_channel::<RconResult<Packet>>();
            let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(1);
            Worker {
                handle: tokio::spawn(RconClient::tcp_read_loop(
                    tcp_read,
                    tx_responses,
                    tx_nonresponses,
                    shutdown_rx,
                )),
                x: rx_responses,
                shutdown: shutdown_tx,
            }
        };

        loop {
            tokio::select! {
                // queries from inside.
                // send packets to the outside.
                query = query_rx.recv() => match query {
                    Some(Query(words, replier)) => {
                        let packet = Packet {
                            sequence,
                            origin: PacketOrigin::Client,
                            is_response: false,
                            words,
                        };
                        waiting.insert(sequence, Waiting {
                            replier,
                            sent: std::time::Instant::now(),
                        });
                        sequence += 1;

                        let bytes = packet.serialize();
                        match tcp_write.write(&bytes.as_slice()).await {
                            Ok(n) if n == bytes.len() => {},
                            Ok(_) /* otherwise */     => panic!("Failed to send packet in its entirety"), // TODO potentially better error handling.
                            Err(e) => {
                                println!("debg [RconClient::mainloop] Got error while writing to socket: {:?}", e);
                                break;
                            },
                        }
                    },
                    None => {
                        break;
                    }
                },
                // packets from the outside
                // reply to queries on the inside, or invoke onKill events etc.
                opt_pack = tcp_in.x.recv() => match opt_pack {
                    Some(Ok(packet)) => {
                        if packet.is_response {
                            if let Some(waiter) = waiting.remove(&packet.sequence) {
                                // wake up the waiting `query()` function.
                                let response_time = std::time::Instant::now().duration_since(waiter.sent).as_millis();
                                // rcon_response_times.push()
                                if response_time > 333 { // 333ms
                                    println!("response time is high: {}ms", response_time);
                                }
                                waiter.replier.send(Ok(packet.words)).expect("Query issuer no longer wants the result?"); // FIXME: this shouldn't panic. Handle error instead.
                            } else {
                                // just ignore it then.
                                println!("warn [RconClient::mainloop] Received a response to a packet which was never a request. Maybe timed out? Packet = {}", packet);
                            }
                        } else {
                            todo!("handle non-response packets.")
                        }
                    },
                    None => {
                        // end of stream, e.g. graceful connection shutdown.
                        break;
                    },
                    Some(Err(e)) if std::mem::discriminant(&e) == std::mem::discriminant(&RconError::ConnectionClosed) => {
                        // end of stream, but not very graceful connection shutdown.
                        println!("warn [RconClient::mainloop] Tcp read loop ungracefully closed connection: {:?}", e);
                        break;
                    },
                    Some(Err(e)) => {
                        // some other error, e.g. malformed packet received.
                        println!("warn [RconClient::mainloop] Tcp read loop ended with error: {:?}", e);
                        break;
                    },
                },
                some = shutdown.recv() => match some {
                    Some(()) => {
                        // println!("     [RconClient::mainloop] Received shutdown signal.");
                        break;
                    },
                    None => {
                        println!("warn [RconClient::mainloop] Received shutdown signal (only because all Sender halves have dropped).");
                    }
                }
            }
        }

        let _ = tcp_in.shutdown.send(()).await; // if we get a SendError that is fine, then tcp reader is simply already closed.
        tcp_in.handle.await.expect("Failed to join tcp_in worker on shutdown. This is a bug. Most likely, the worker panicked.");

        // accept no more queries, and send error to any still-waiting queries.
        query_rx.close();
        for (_, w) in waiting.drain() {
            w.replier.send(Err(RconError::ConnectionClosed)).unwrap();
        }
        // we don't accept new queries, but it is possible a query was sent before that and not seen by us yet.
        while let Some(Query(_, tx)) = query_rx.recv().await {
            tx.send(Err(RconError::ConnectionClosed)).unwrap();
        }

        // println!("     [RconClient::mainloop] Ended gracefully");
    }

    pub async fn query_raw(&self, words: &Vec<AsciiString>) -> RconResult<Vec<AsciiString>> {
        let (tx, rx) = oneshot::channel::<RconResult<Vec<AsciiString>>>();

        self.queries
            .send(Query(words.clone(), tx))
            .map_err(|_: mpsc::error::SendError<_>| RconError::ConnectionClosed)?; // when mainloop did `rx.close()` at the end for example.
        rx.await.expect(
            "query_raw: failed to receive query response from main loop. This is likely a bug.",
        )
    }

    pub async fn query<A>(&self, words: impl IntoIterator<Item = A>) -> RconResult<Vec<AsciiString>>
    where
        A: IntoAsciiString,
    {
        // convert each word to ascii.
        let mut words_ascii = Vec::new();
        for w in words.into_iter().map(|w| w.into_ascii_string()) {
            words_ascii.push(w?);
        }

        self.query_raw(&words_ascii).await
    }

    pub async fn command<T, E>(
        &self,
        words: &Vec<AsciiString>,
        ok: impl FnOnce(&Vec<AsciiString>) -> Result<T, E>,
        err: impl FnOnce(&str) -> Option<E>,
    ) -> Result<T, E>
    where
        E: From<RconError>,
    {
        let res = self.query_raw(words).await?;
        match res[0].as_str() {
            "OK" => ok(&res),
            "UnknownCommand" => Err(RconError::UnknownCommand.into()),
            "InvalidArguments" => Err(RconError::InvalidArguments.into()),
            word => Err(err(word).unwrap_or(RconError::UnknownResponse.into())),
        }
    }

    pub async fn events_enabled(&self, enabled: bool) -> RconResult<()> {
        // there exists a get version of this, but I assume it'll be never needed.
        self.command(
            &veca!["admin.eventsEnabled", enabled.to_string()],
            ok_eof,
            err_none,
        )
        .await
    }

    // pub async fn shutdown(&mut self) -> RconResult<()> {
    //     println!("rcon shutdown invoked");
    //     self.shutdown_tx.send(()).unwrap();
    //     // maybe better error handling some day... sigh...
    //     (&mut self.mainloop).await.unwrap()?;

    //     // this is technically wrong. We shouldn't await the mainloop JoinHandle twice I think, but this
    //     // atomic store doesn't prevent that. Need a proper mutex I think. But alas, too lazy, it'll be fiiiiine.
    //     self.drop_ready
    //         .store(true, std::sync::atomic::Ordering::SeqCst);

    //     Ok(())
    // }
}

/// Use this to assert that there is no more extra input. As in, we only expect
/// the first word to be "OK" (already checked at a different place),
/// and nothing else.
/// Basically just a convenience function.
pub(crate) fn ok_eof<E>(words: &Vec<AsciiString>) -> Result<(), E>
where
    E: From<RconError>,
{
    if words.len() == 1 {
        Ok(())
    } else {
        Err(RconError::ProtocolError.into())
    }
}

pub(crate) fn err_none<E>(_errorcode: &str) -> Option<E>
where
    E: From<RconError>,
{
    None
}

impl Drop for RconClient {
    fn drop(&mut self) {
        let _ = self.mainloop_shutdown.send(()).unwrap(); // if we get a SendError, that means the main loop already dropped its Receiver. So the .unwrap() might cause more damage than safety. But for now, I'm leaving it here until we run into issues.
        if self.mainloop.is_some() {
            self.mainloop
                .take()
                .unwrap()
                .join()
                .expect("[RconClient::drop] Could not join mainloop");
        }
    }
}
