use core::panic;
use std::{collections::HashMap, convert::TryInto, io::ErrorKind, str::FromStr, sync::atomic::AtomicBool};

// use crate::error::{Error, Result};
use ascii::{AsciiString, FromAsciiError, IntoAsciiString};
use packet::{Packet, PacketDeserializeResult, PacketOrigin};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::{
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
    sync::{broadcast, mpsc, oneshot},
};
use tokio::{net::TcpStream, task::JoinHandle};

pub(crate) mod packet;

#[derive(Debug)]
pub enum RconError {
    /// Prominently when ip:port are wrong, etc.
    Io(std::io::Error),

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
    /// When *we* don't know what the fuck rcon just responded to us.
    UnknownResponse,
}

impl From<std::io::Error> for RconError {
    fn from(e: std::io::Error) -> Self {
        RconError::Io(e)
    }
}

impl<T> From<FromAsciiError<T>> for RconError {
    fn from(_: FromAsciiError<T>) -> Self {
        RconError::NotAscii
    }
}

pub type RconResult<T> = Result<T, RconError>;

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

pub struct RconClient {
    mainloop: JoinHandle<RconResult<()>>,
    mainloop_ctrl: mpsc::UnboundedSender<Query>,
    shutdown_tx: broadcast::Sender<()>,

    _connection_info: RconConnectionInfo,

    drop_ready: AtomicBool,
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
    pub async fn connect(conn: impl Into<RconConnectionInfo>, events_caller: impl RconEventPacketHandler + Send + 'static) -> RconResult<Self>
    {
        let conn : RconConnectionInfo = conn.into();
        // println!("uhhh, connecting to {}:{}", ip, port);
        let tcp = TcpStream::connect((conn.ip.clone(), conn.port)).await?;
        // println!("ahhhh");

        let (tx, rx) = mpsc::unbounded_channel::<Query>();
        let (shutdown_tx, _) = broadcast::channel::<()>(4);

        let mainloop = tokio::spawn(RconClient::mainloop(rx, tcp, shutdown_tx.clone(), events_caller));

        let myself = RconClient {
            mainloop,
            mainloop_ctrl: tx.clone(),
            shutdown_tx: shutdown_tx.clone(),

            _connection_info: RconConnectionInfo {
                ip: conn.ip,
                port: conn.port,
                password: conn.password.clone(),
            },
            drop_ready: AtomicBool::new(false),
        };

        // at this point we should have a fully functional async way to query.
        // so we just login and set stuff up, and done!

        // TODO: use salted passwords eventually.
        let result = myself
            .query_raw(vec![
                "login.plainText".into_ascii_string().unwrap(),
                conn.password,
            ])
            .await; // Err: Many.. TODO
        // println!("Got login response: {:?}", result);

        Ok(myself)
    }

    async fn tcp_write_loop(
        mut rx: mpsc::UnboundedReceiver<Packet>,
        mut tcp: OwnedWriteHalf,
        shutdown_tx: broadcast::Sender<()>,
        mut shutdown_rx: broadcast::Receiver<()>,
    ) -> RconResult<()> {
        // while let Some(packet) = rx.recv().await {
        //     let bytes = packet.serialize();
        //     if tcp.write(&bytes.as_slice()).await? != bytes.len() {
        //         panic!("Failed to write buf in tcp_write_loop");
        //     }
        // }

        loop {
            // let start = std::time::Instant::now();
            tokio::select! {
                somepacket = rx.recv() => match somepacket {
                    Some(packet) => {
                        // println!("tcp_write_loop packet receiving time: {}micros", std::time::Instant::now().duration_since(start).as_micros());
                        // println!("Out: {}", packet);
                        let bytes = packet.serialize();
                        if tcp.write(&bytes.as_slice()).await? != bytes.len() {
                            panic!("Failed to write buf in tcp_write_loop");
                        }
                    },
                    None => break, // end of stream, graceful shutdown
                },
                _ = shutdown_rx.recv() => {
                    // not sure if we should finish sending the rest of the packets...
                    // or just break here...
                    // println!("tcp_write_loop received shutdown signal. Doing rx.close().");
                    rx.close();
                },
            }
        }

        shutdown_tx.send(()).expect(
            "This is some kinda bug. Couldn't send shutdown signal at end of tcp_write_loop.",
        );

        // println!("tcp_write_loop ended gracefully");
        Ok(())
    }

    async fn tcp_read_loop(
        tx: mpsc::UnboundedSender<Packet>,
        mut tcp: OwnedReadHalf,
        shutdown_tx: broadcast::Sender<()>,
        mut shutdown_rx: broadcast::Receiver<()>,
        events_caller: impl RconEventPacketHandler,
    ) -> RconResult<()> {
        let mut buf = vec![0_u8; 12]; // header size. We'll grow the buffer as necessary later.

        'outer: loop {
            // let start = Instant::now();
            tokio::select! {
                // read 12 byte header
                tcpread = tcp.read_exact(&mut buf[0..12]) => {
                    // println!("Time waiting+reading 12byte header: {}micros", Instant::now().duration_since(start).as_micros());

                    // make sure the read was successful
                    match tcpread {
                        Ok(n) if n == 0 => {
                            // bus.tx.send(AcrossBroadcast::TcpClosed).expect("Internal error, this is a BUG. Tcp stream ended, but could not broadcast the message.");
                            shutdown(&shutdown_tx).await;
                            return Err(RconError::ConnectionClosed);
                        },
                        Ok(n) => {
                            assert_eq!(n, 12);
                        },
                        Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                            // bus.tx.send(AcrossBroadcast::TcpClosed).expect("Internal error, this is a BUG. Tcp stream ended, but could not broadcast the message.");
                            shutdown(&shutdown_tx).await;
                            return Err(RconError::ConnectionClosed);
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
                    loop {
                        tokio::select! {
                            tcpread = tcp.read_exact(&mut buf[12..total_len]) => {
                                // make sure the read was successful
                                match tcpread {
                                    Ok(n) if n == 0 => {
                                        // bus.tx.send(AcrossBroadcast::TcpClosed).expect("Internal error, this is a BUG. Tcp stream ended, but could not broadcast the message.");
                                        shutdown(&shutdown_tx).await;
                                        return Err(RconError::ConnectionClosed);
                                    },
                                    Ok(n) => {
                                        assert_eq!(n, total_len - 12);
                                    },
                                    Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                                        // bus.tx.send(AcrossBroadcast::TcpClosed).expect("Internal error, this is a BUG. Tcp stream ended, but could not broadcast the message.");
                                        shutdown(&shutdown_tx).await;
                                        return Err(RconError::ConnectionClosed);
                                    },
                                    Err(e) => {
                                        panic!("unexpected io error in tcp reader: {:?}", e);
                                    }
                                };

                                let packet = match Packet::deserialize(&buf[0..total_len]) {
                                    PacketDeserializeResult::Ok {packet, consumed_bytes} => {
                                        if consumed_bytes != total_len {
                                            // bus.tx.send(AcrossBroadcast::TcpClosed).expect("Internal error, this is a BUG. Received malformed packet, but could not broadcast the message.")
                                            shutdown(&shutdown_tx).await;
                                            return Err(RconError::ProtocolError);
                                        }
                                        packet
                                    },
                                    _ => {
                                        // bus.tx.send(AcrossBroadcast::TcpClosed).expect("Internal error, this is a BUG. Received malformed packet, but could not broadcast the message.");
                                        shutdown(&shutdown_tx).await;
                                        return Err(RconError::ProtocolError);
                                    }
                                };

                                // println!("In:  {}", packet);
                                if packet.is_response {
                                    // this send should NEVER fail until mainloop gets dropped.
                                    // which it won't, because it's waiting to join this thread.
                                    // ...unless it panics, then we're all doomed.
                                    tx.send(packet).expect("[tcp_read_loop] Internal error, this is a BUG. Could not send QueryResponse message.");
                                    break; // break inner loop => read next header for next packet.
                                } else {
                                    // this will give the packet to the mainloop (kinda),
                                    // which will give it to Bf4Client,
                                    // which will read it and convert strings to types and then call its events_caller but then with a Bf4Event.
                                    events_caller.on_packet(packet);
                                    break;
                                    // todo!("Need to still implement normal non-reply packets hehe")
                                }
                            },
                            _ = shutdown_rx.recv() => {
                                println!("warn [Rcon tcp_read_loop] received shutdown signal, but had a packet partially read.");
                                break 'outer;
                            }
                        }
                    }
                },
                _ = shutdown_rx.recv() => break,
            }
        }

        shutdown(&shutdown_tx).await;
        // shutdown_tx.send(()).expect(
        //     "This is some kinda bug. Couldn't send shutdown signal at end of tcp_read_loop.",
        // );

        async fn shutdown(tx: &broadcast::Sender<()>) {
            // println!("[tcp_read_loop] Sending shutdown signal..");
            tx.send(()).expect("[tcp_read_loop] This is a bug. Could not send shutdown signal");
        }

        // println!("tcp_read_loop ended gracefully");
        Ok(())
        // we drop the TCP half here.
    }

    async fn mainloop(
        mut rx: mpsc::UnboundedReceiver<Query>,
        tcp: TcpStream,
        shutdown_tx: broadcast::Sender<()>,
        events_caller: impl RconEventPacketHandler + Send + 'static,
    ) -> RconResult<()> {
        // no need for mutexes locking the sequence numbers and `waiting`, since we're using message passing.
        struct Waiting {
            replier: oneshot::Sender<RconResult<Vec<AsciiString>>>,
            sent: std::time::Instant,
        }

        let mut sequence: u32 = 0;
        let mut waiting: HashMap<u32, Waiting> = HashMap::new();
        // let mut rcon_response_times = VecDeque::new();
        let (tcp_read, tcp_write) = tcp.into_split();

        // workers. I am not sure how to make this cleaner. guess I could simply make it single-threaded/-tasked... but oh well, too late.

        struct Worker<T> {
            handle: JoinHandle<RconResult<()>>,
            x: T,
        }
        let tcp_out = {
            let (tcp_out_tx, tcp_out_rx) = mpsc::unbounded_channel::<Packet>();
            Worker {
                handle: tokio::spawn(RconClient::tcp_write_loop(
                    tcp_out_rx,
                    tcp_write,
                    shutdown_tx.clone(),
                    shutdown_tx.subscribe(),
                )),
                x: tcp_out_tx,
            }
        };
        let mut tcp_in = {
            let (tcp_in_tx, tcp_in_rx) = mpsc::unbounded_channel::<Packet>();
            Worker {
                handle: tokio::spawn(RconClient::tcp_read_loop(
                    tcp_in_tx,
                    tcp_read,
                    shutdown_tx.clone(),
                    shutdown_tx.subscribe(),
                    events_caller,
                )),
                x: tcp_in_rx,
            }
        };

        let mut shutdown_rx = shutdown_tx.subscribe();
        loop {
            tokio::select! {
                // queries from inside.
                // send packets to the outside.
                query = rx.recv() => match query {
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

                        // ignore the result of send. If it fails, means tcp_out has died, aka connection has been lost
                        // then we'll simply catch tcp_out.handle in the next loop select here. And then we will stop too.
                        // println!("mainloop: tcp_out.x.send(packet).unwrap(); with packet = {}", packet);
                        tcp_out.x.send(packet).unwrap();
                        // FIXME: actually, that means some queries are still waiting on their oneshot result and will never get it... ISSUE!
                    },
                    None => {
                        break;
                    }
                },
                // packets from the outside
                // reply to queries on the inside, or invoke onKill events etc.
                packet = tcp_in.x.recv() => match packet {
                    Some(packet) => {
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
                                println!("Received a response to a packet which was never a request. Maybe timed out? Packet = {}", packet);
                            }
                        } else {
                            todo!("handle non-response packets.")
                        }
                    },
                    None => {
                        break;
                    },
                },
                _ = shutdown_rx.recv() => {
                    println!("mainloop: Received shutdown signal.");
                    break;
                }
            }
        }

        shutdown_tx.send(()).unwrap();
        tcp_in.handle.await.expect("Failed to join tcp_in worker on shutdown. This is a bug. Most likely, the worker panicked.")?;
        tcp_out.handle.await.expect("Failed to join tcp_out worker on shutdown. This is a bug. Most likely, the worker panicked.")?;

        // println!("mainloop ended gracefully");
        Ok(())
    }

    pub async fn query_raw(&self, words: Vec<AsciiString>) -> RconResult<Vec<AsciiString>> {
        let (tx, rx) = oneshot::channel::<RconResult<Vec<AsciiString>>>();

        self.mainloop_ctrl
            .send(Query(words, tx))
            .expect("query_raw: failed to send query message to main loop. This is likely a bug.");
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

        self.query_raw(words_ascii).await
    }

    pub async fn command<T, E>(
        &self,
        words: Vec<AsciiString>,
        ok: impl FnOnce(Vec<AsciiString>) -> Result<T, E>,
        err: impl FnOnce(&str) -> Option<E>,
    ) -> Result<T, E>
    where
        E: From<RconError>,
    {
        let res = self.query_raw(words).await?;
        match res[0].as_str() {
            "OK" => ok(res),
            "UnknownCommand" => Err(RconError::UnknownCommand.into()),
            "InvalidArguments" => Err(RconError::InvalidArguments.into()),
            word => Err(err(word).unwrap_or(RconError::UnknownResponse.into())),
        }
    }

    pub async fn events_enabled(&self, enabled: bool) -> RconResult<()> {
        // there exists a get version of this, but I assume it'll be never needed.
        self.command(veca!["admin.eventsEnabled", enabled.to_string()], ok_eof, err_none,).await
    }

    


    pub async fn shutdown(&mut self) -> RconResult<()> {
        println!("rcon shutdown invoked");
        self.shutdown_tx.send(()).unwrap();
        // maybe better error handling some day... sigh...
        (&mut self.mainloop).await.unwrap()?;

        // this is technically wrong. We shouldn't await the mainloop JoinHandle twice I think, but this
        // atomic store doesn't prevent that. Need a proper mutex I think. But alas, too lazy, it'll be fiiiiine.
        self.drop_ready
            .store(true, std::sync::atomic::Ordering::SeqCst);

        Ok(())
    }
}

/// Use this to assert that there is no more extra input. As in, we only expect
/// the first word to be "OK" (already checked at a different place),
/// and nothing else.
/// Basically just a convenience function.
pub(crate) fn ok_eof<E>(words: Vec<ascii::AsciiString>) -> Result<(), E>
    where E: From<RconError>
{
    if words.len() == 1 {
        Ok(())
    } else {
        Err(RconError::ProtocolError.into())
    }
}

pub(crate) fn err_none<E>(_errorcode: &str) -> Option<E>
    where E: From<RconError>
{
    None
}

impl Drop for RconClient {
    fn drop(&mut self) {
        // Ugh, no async drops. This is terrible and hacky.
        if !self.drop_ready.load(std::sync::atomic::Ordering::SeqCst) {
            println!("Warning: RconClients must be .shutdown() before they can be dropped!");
        }
    }
}
