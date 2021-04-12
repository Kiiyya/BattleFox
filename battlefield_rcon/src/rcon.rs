use core::panic;
use std::{collections::HashMap, convert::TryInto, io::ErrorKind, num::ParseIntError, sync::Arc};

// use crate::error::{Error, Result};
use ascii::{AsciiString, FromAsciiError, IntoAsciiString};
use packet::{Packet, PacketDeserializeResult, PacketOrigin};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::ToSocketAddrs};
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
    ProtocolError(Option<String>),

    /// Some string passed into this api could not be converted to Ascii.
    /// E.g. contains utf8 characters which don't exist in Ascii.
    NotAscii(String),

    /// Some rcon query/command timed out.
    TimedOut,

    /// When Rcon says this command does not exist.
    UnknownCommand { our_query: Vec<AsciiString> },
    /// When Rcon says that something's wrong with the arguments to the command.
    InvalidArguments { our_query: Vec<AsciiString> },
    /// When *we* don't know what the fuck rcon just responded to us. More like unknown query response.
    UnknownResponse {
        our_query: Vec<AsciiString>,
        rcon_response: Vec<AsciiString>,
    },
    /// When *we* don't understand the arguments of a response / event.
    MalformedPacket {
        words: Vec<AsciiString>,
        explanation: Option<String>,
    },

    /// Some rare or very weird error.
    Other(String),
}

impl RconError {
    pub fn malformed_packet(
        words: impl IntoIterator<Item = AsciiString>,
        explanation: impl Into<String>,
    ) -> Self {
        Self::MalformedPacket {
            words: words.into_iter().collect(),
            explanation: Some(explanation.into()),
        }
    }

    pub fn other(str: impl Into<String>) -> Self {
        RconError::Other(str.into())
    }

    pub fn protocol_msg(str: impl Into<String>) -> Self {
        // let str : String = str.into();
        // if str.is_empty() {
        //     Self::ProtocolError(None)
        // } else {
        Self::ProtocolError(Some(str.into()))
        // }
    }

    pub fn protocol() -> Self {
        Self::ProtocolError(None)
    }
}

impl From<std::io::Error> for RconError {
    fn from(e: std::io::Error) -> Self {
        RconError::Io(Arc::new(e))
    }
}

impl<T: Into<String>> From<FromAsciiError<T>> for RconError {
    fn from(err: FromAsciiError<T>) -> Self {
        RconError::NotAscii(err.into_source().into())
    }
}

// impl From<FromAsciiError<AsciiString>> for RconError {
//     fn from(err: FromAsciiError<AsciiString>) -> Self {
//         RconError::NotAscii(err.into_source().into())
//     }
// }

pub type RconResult<T> = Result<T, RconError>;

#[derive(Debug, Clone)]
pub struct RconConnectionInfo {
    pub ip: String,
    pub port: u16,
    pub password: AsciiString,
}

// impl Into<RconConnectionInfo> for (String, u16, AsciiString) {
//     fn into(self) -> RconConnectionInfo {
//         RconConnectionInfo {
//             ip: self.0,
//             port: self.1,
//             password: self.2,
//         }
//     }
// }

// impl<'a> TryInto<RconConnectionInfo> for (&str, u16, &str) {
//     type Error = RconError;
//     fn try_into(self) -> std::result::Result<RconConnectionInfo, RconError> {
//         Ok(RconConnectionInfo {
//             ip: self.0.into(),
//             port: self.1,
//             password: AsciiString::from_str(self.2)
//                 .map_err(|_| RconError::NotAscii(self.2.to_string()))?,
//         })
//     }
// }

#[derive(Debug)]
pub struct RconClient {
    /// So that we can drop the rconclient later.
    mainloop: Option<std::thread::JoinHandle<()>>,
    /// In order to drop drop the rconclient, we need to tell the mainloop to stop.
    mainloop_shutdown: mpsc::UnboundedSender<()>,

    /// Sending a query to this will be handled by the mainloop.
    queries: mpsc::UnboundedSender<SendQuery>,

    nonresponse_rx: Option<mpsc::UnboundedReceiver<RconResult<Packet>>>,

    // / ip, port, password.
    // _connection_info: RconConnectionInfo,
}

// #[derive(Debug)]
// struct Query(
//     Vec<AsciiString>,
//     oneshot::Sender<RconResult<Vec<AsciiString>>>,
// );

#[derive(Debug)]
struct SendSingleQuery(
    Vec<AsciiString>,
    oneshot::Sender<RconResult<Vec<AsciiString>>>,
);

#[derive(Debug)]
enum SendQuery {
    Single(SendSingleQuery),
    Sequential(Vec<SendSingleQuery>),
}

pub trait RconEventPacketHandler {
    fn on_packet(&self, packet: Packet);
}

/// Just used internally to do a remote procedure call.
impl RconClient {
    #[allow(clippy::useless_vec)]
    pub async fn connect(addr: impl ToSocketAddrs) -> RconResult<Self> {
        let tcp = TcpStream::connect(addr).await?;

        let (query_tx, query_rx) = mpsc::unbounded_channel::<SendQuery>();
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
        };

        // at this point we should have a fully functional async way to query.

        Ok(myself)
    }

    #[allow(clippy::useless_vec)]
    pub async fn login_hashed(&self, password: impl IntoAsciiString + Into<String>) -> RconResult<()> {
        let password = password.into_ascii_string()?;

        let salt = self
            .query(
                &veca!["login.hashed"],
                |ok| {
                    if ok.len() != 1 {
                        Err(RconError::protocol_msg(format!(
                            "Expected one return value (the salt), but got {} instead!",
                            ok.len()
                        )))
                    } else {
                        Ok(decode_hex(ok[0].as_str()).map_err(|_| {
                            RconError::protocol_msg("Server replied with an invalid hash")
                        })?)
                    }
                },
                |err| match err {
                    "PasswordNotSet" => Some(RconError::other(
                        "The server has no password set. Login is impossible.",
                    )),
                    _ => None,
                },
            )
            .await?;

        let mut hash_in = salt;
        hash_in.extend_from_slice(password.as_bytes());
        let hash = md5::compute(hash_in);

        self
            .query(&veca!["login.hashed", format!("{:?}", hash).to_ascii_uppercase()], ok_eof, |err| match err {
                "InvalidPasswordHash" => Some(RconError::WrongPassword),
                "PasswordNotSet" => Some(RconError::other(
                    "The server has no password set. Login is impossible.",
                )),
                _ => None,
            })
            .await?;

        Ok(())
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
                                            ret = Err(RconError::protocol_msg("Malformed packet received"));
                                            break 'outer;
                                        }
                                        packet
                                    },
                                    _ => {
                                        ret = Err(RconError::protocol());
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
        mut query_rx: mpsc::UnboundedReceiver<SendQuery>,
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
                    Some(SendQuery::Single(SendSingleQuery(words, replier))) => { //Query(words, replier)) => {
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
                    Some(SendQuery::Sequential(queries)) => {
                        for SendSingleQuery(words, replier) in queries {
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
                                if response_time > 2000 { // 333ms
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
                        // println!("warn [RconClient::mainloop] Tcp read loop ungracefully closed connection: {:?}", e);
                        // At least on nitrado, this is what happens when you shut the server down via `admin.shutdown` rcon.
                        // So I'm removing it from the "ungraceful" status.
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
        while let Some(SendQuery::Single(SendSingleQuery(_, tx))) = query_rx.recv().await {
            tx.send(Err(RconError::ConnectionClosed)).unwrap();
        }

        // println!("     [RconClient::mainloop] Ended gracefully");
    }

    // #[allow(clippy::useless_vec)]
    pub async fn events_enabled(&self, enabled: bool) -> RconResult<()> {
        // there exists a get version of this, but I assume it'll be never needed.
        self.query(
            veca!["admin.eventsEnabled", enabled.to_string()].as_slice(),
            ok_eof,
            |_| None,
        )
        .await
    }

    // /// Multiple queries, guaranteed to be sent in order.
    // /// This is (slightly) faster than doing multiple queries and awaiting each.
    // ///
    // /// Sends all queries immediately, so if the first query returns an error,
    // /// the next queries will have already been sent.
    // pub async fn queries<T, E>(
    //     &self,
    // )
}

// #[derive(Debug)]
// pub struct Queries {

// }

#[async_trait::async_trait]
pub trait RconQueryable {
    /// Send single query and await response, returning the words.
    async fn query_raw(&self, words: Vec<AsciiString>) -> RconResult<Vec<AsciiString>>;

    /// Send multiple queries, guaranteeing that they will be sent in that order.
    /// Then awaits responses, and returns them.
    ///
    /// # Returns
    /// - `None`: Some error occured communicating with the main loop. This likely means connection was closed.
    /// - `Some(vec)`: Each item contains a `RconResult<Vec<AsciiString>>`, as if it was just multiple calls to `query_raw`.
    async fn queries_raw(
        &self,
        words: Vec<Vec<AsciiString>>,
    ) -> Option<Vec<RconResult<Vec<AsciiString>>>>;

    /// More convenient way than `query_raw`.
    async fn query<T, E, Ok, Err>(&self, words: &[AsciiString], ok: Ok, err: Err) -> Result<T, E>
    where
        E: From<RconError>,
        Ok: FnOnce(&[AsciiString]) -> Result<T, E> + Send,
        Err: FnOnce(&str) -> Option<E> + Send,
    {
        // println!("Query::Single out: {:?}", words);
        let res = self.query_raw(words.to_vec()).await?;
        match res[0].as_str() {
            "OK" => ok(&res[1..]),
            "UnknownCommand" => Err(RconError::UnknownCommand {
                our_query: words.to_vec(),
            }
            .into()),
            "InvalidArguments" => Err(RconError::InvalidArguments {
                our_query: words.to_vec(),
            }
            .into()),
            word => Err(err(word).unwrap_or_else(|| {
                RconError::UnknownResponse {
                    our_query: words.to_vec(),
                    rcon_response: res.clone(),
                }
                .into()
            })),
        }
    }
}

#[async_trait::async_trait]
impl RconQueryable for RconClient {
    async fn query_raw(&self, words: Vec<AsciiString>) -> RconResult<Vec<AsciiString>> {
        let (tx, rx) = oneshot::channel::<RconResult<Vec<AsciiString>>>();

        self.queries
            .send(SendQuery::Single(SendSingleQuery(words, tx)))
            .map_err(|_: mpsc::error::SendError<_>| RconError::ConnectionClosed)?; // when mainloop did `rx.close()` at the end for example.
        rx.await.expect(
            "query_raw: failed to receive query response from main loop. This is likely a bug.",
        )
    }

    async fn queries_raw(
        &self,
        wordses: Vec<Vec<AsciiString>>,
    ) -> Option<Vec<RconResult<Vec<AsciiString>>>> {
        let mut single_queries = Vec::new();
        let mut waiting = Vec::new();

        for words in wordses {
            let (tx, rx) = oneshot::channel::<RconResult<Vec<AsciiString>>>();
            single_queries.push(SendSingleQuery(words, tx));
            waiting.push(rx);
        }

        if self
            .queries
            .send(SendQuery::Sequential(single_queries))
            .is_err()
        {
            return None;
        }

        let mut result = Vec::new();
        for rx in waiting {
            let res = rx.await.expect(
                "queries_raw: failed to receive query response from main loop. This is likely a bug.",
            );
            result.push(res);
        }

        Some(result)
    }
}

/// Use this to assert that there is no more extra input. As in, we only expect
/// the first word to be "OK" (already checked at a different place),
/// and nothing else.
/// Basically just a convenience function.
pub(crate) fn ok_eof<E>(words: &[AsciiString]) -> Result<(), E>
where
    E: From<RconError>,
{
    if words.is_empty() {
        Ok(())
    } else {
        Err(RconError::protocol().into())
    }
}

impl Drop for RconClient {
    fn drop(&mut self) {
        let _ = self.mainloop_shutdown.send(()); // ignore potential SendError. If the TCP connection gets closed by remote, mainloop will `break;` and close/drop the shutdown receirver, thus the send here might actually fail. But in that case, we can ignore it, since that's what we want anyway.
        if self.mainloop.is_some() {
            self.mainloop
                .take()
                .unwrap()
                .join()
                .expect("[RconClient::drop] Could not join mainloop");
        }
    }
}

fn decode_hex(s: &str) -> Result<Vec<u8>, ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}
