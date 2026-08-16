use std::{
    collections::HashMap,
    io::{self, Read, Write},
    net::SocketAddr,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

use bytes::{Buf, BytesMut};
use mio::net::{TcpListener, TcpStream, UdpSocket};
use mio::{Events, Interest, Poll, Token, Waker};

use crate::http2::{Http2Connection, HTTP2_PREFACE};
use crate::http3::Http3Connection;
use crate::{error::HttpError, handler, request::Request};

const WAKER_TOKEN: Token = Token(0);
const IDLE_TIMEOUT: Duration = Duration::from_secs(10);
const BUFFER_CAPACITY: usize = 4096;

struct SubReactorHandle {
    sender: Sender<TcpStream>,
    waker: Arc<Waker>,
}

enum ProtocolState {
    Unknown,
    Http1,
    Http2(Http2Connection),
}

struct ClientConnection {
    stream: TcpStream,
    read_buf: BytesMut,
    write_buf: BytesMut,
    last_activity: Instant,
    protocol: ProtocolState,
}

struct SubReactor {
    id: usize,
    poll: Poll,
    rx: Receiver<TcpStream>,
    clients: HashMap<Token, ClientConnection>,
}

impl SubReactor {
    fn new(id: usize, rx: Receiver<TcpStream>) -> Result<(Self, Arc<Waker>), HttpError> {
        let poll = Poll::new()?;
        let waker = Arc::new(Waker::new(poll.registry(), WAKER_TOKEN)?);
        Ok((
            Self {
                id,
                poll,
                rx,
                clients: HashMap::new(),
            },
            waker,
        ))
    }

    fn run(mut self) {
        let mut events = Events::with_capacity(1024);
        let mut token_counter: usize = 1;
        let mut last_cleanup = Instant::now();

        loop {
            if let Err(e) = self.poll.poll(&mut events, Some(Duration::from_millis(50))) {
                eprintln!("sub-reactor {} poll error: {e}", self.id);
                continue;
            }

            for event in events.iter() {
                match event.token() {
                    WAKER_TOKEN => {
                        while let Ok(mut stream) = self.rx.try_recv() {
                            let token = Token(token_counter);
                            token_counter += 1;

                            if let Err(e) = self.poll.registry().register(
                                &mut stream,
                                token,
                                Interest::READABLE,
                            ) {
                                eprintln!("failed to register stream: {e}");
                                continue;
                            }

                            self.clients.insert(
                                token,
                                ClientConnection {
                                    stream,
                                    read_buf: BytesMut::with_capacity(BUFFER_CAPACITY),
                                    write_buf: BytesMut::with_capacity(BUFFER_CAPACITY),
                                    last_activity: Instant::now(),
                                    protocol: ProtocolState::Unknown,
                                },
                            );
                        }
                    }
                    token => {
                        self.handle_client_event(token, event);
                    }
                }
            }

            if last_cleanup.elapsed() >= Duration::from_secs(2) {
                self.prune_idle_connections();
                last_cleanup = Instant::now();
            }
        }
    }

    fn handle_client_event(&mut self, token: Token, event: &mio::event::Event) {
        let mut should_remove = false;

        if let Some(client) = self.clients.get_mut(&token) {
            client.last_activity = Instant::now();

            // 1. Flush write buffer when socket is writable
            if event.is_writable() && !client.write_buf.is_empty() {
                match client.stream.write(&client.write_buf) {
                    Ok(n) => {
                        client.write_buf.advance(n);
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {}
                    Err(_) => {
                        should_remove = true;
                    }
                }
            }

            // 2. Read incoming data when socket is readable
            if event.is_readable() && !should_remove {
                let mut temp_buf = [0u8; 2048];
                loop {
                    match client.stream.read(&mut temp_buf) {
                        Ok(0) => {
                            should_remove = true;
                            break;
                        }
                        Ok(n) => {
                            client.read_buf.extend_from_slice(&temp_buf[..n]);

                            // Determine protocol if unknown
                            if matches!(client.protocol, ProtocolState::Unknown) {
                                if Http2Connection::is_http2_preface(&client.read_buf) {
                                    client.protocol = ProtocolState::Http2(Http2Connection::new());
                                } else if client.read_buf.len() >= HTTP2_PREFACE.len()
                                    || client.read_buf.windows(4).any(|w| w == b"\r\n\r\n")
                                {
                                    client.protocol = ProtocolState::Http1;
                                }
                            }

                            // Process based on detected protocol
                            match &mut client.protocol {
                                ProtocolState::Http2(h2_conn) => {
                                    match h2_conn.process_input(&client.read_buf) {
                                        Ok(Some((consumed, completed_requests))) => {
                                            client.read_buf.advance(consumed);
                                            for (stream_id, req) in completed_requests {
                                                let res = handler::route(&req);
                                                let res_bytes =
                                                    Http2Connection::encode_response(stream_id, &res);
                                                client.write_buf.extend_from_slice(&res_bytes);
                                            }
                                        }
                                        Ok(None) => {}
                                        Err(_) => {
                                            should_remove = true;
                                            break;
                                        }
                                    }
                                }
                                ProtocolState::Http1 => {
                                    match Request::parse(&client.read_buf) {
                                        Ok(Some((req, consumed_bytes))) => {
                                            let mut res = handler::route(&req);
                                            let should_close = res.apply_connection_header(&req);

                                            let mut res_bytes = Vec::new();
                                            if let Err(e) = res.write_to_stream(&mut res_bytes) {
                                                eprintln!("response render error: {e}");
                                                should_remove = true;
                                                break;
                                            }

                                            client.write_buf.extend_from_slice(&res_bytes);
                                            client.read_buf.advance(consumed_bytes);

                                            if should_close {
                                                should_remove = true;
                                                break;
                                            }
                                        }
                                        Ok(None) => break,
                                        Err(_) => {
                                            should_remove = true;
                                            break;
                                        }
                                    }
                                }
                                ProtocolState::Unknown => {
                                    break;
                                }
                            }

                            // Non-blocking flush attempt
                            if !client.write_buf.is_empty() {
                                match client.stream.write(&client.write_buf) {
                                    Ok(written) => {
                                        client.write_buf.advance(written);
                                    }
                                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {}
                                    Err(_) => {
                                        should_remove = true;
                                        break;
                                    }
                                }
                            }
                        }
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                            break;
                        }
                        Err(_) => {
                            should_remove = true;
                            break;
                        }
                    }
                }
            }

            if !should_remove {
                let interest = if client.write_buf.is_empty() {
                    Interest::READABLE
                } else {
                    Interest::READABLE | Interest::WRITABLE
                };

                let _ = self
                    .poll
                    .registry()
                    .reregister(&mut client.stream, token, interest);
            }
        }

        if should_remove {
            if let Some(mut client) = self.clients.remove(&token) {
                let _ = self.poll.registry().deregister(&mut client.stream);
            }
        }
    }

    fn prune_idle_connections(&mut self) {
        let now = Instant::now();
        let mut expired = Vec::new();

        for (&token, client) in self.clients.iter() {
            if now.duration_since(client.last_activity) >= IDLE_TIMEOUT {
                expired.push(token);
            }
        }

        for token in expired {
            if let Some(mut client) = self.clients.remove(&token) {
                let _ = self.poll.registry().deregister(&mut client.stream);
            }
        }
    }
}

const SERVER_TOKEN: Token = Token(0);
const UDP_SERVER_TOKEN: Token = Token(1);

pub struct Server {
    listener: TcpListener,
    udp_listener: UdpSocket,
    workers: Vec<SubReactorHandle>,
    next_worker: usize,
}

impl Server {
    pub fn new(addr: &str) -> Result<Self, HttpError> {
        let socket_addr: SocketAddr = addr.parse().expect("Invalid address format");
        let listener = TcpListener::bind(socket_addr)?;
        let udp_listener = UdpSocket::bind(socket_addr)?;

        let num_cores = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        let mut workers = Vec::with_capacity(num_cores);

        for id in 0..num_cores {
            let (tx, rx) = channel();
            let (sub_reactor, waker) = SubReactor::new(id, rx)?;

            thread::spawn(move || {
                sub_reactor.run();
            });

            workers.push(SubReactorHandle { sender: tx, waker });
        }

        Ok(Self {
            listener,
            udp_listener,
            workers,
            next_worker: 0,
        })
    }

    pub fn run(mut self) {
        let mut poll = Poll::new().expect("failed to create Master Poll instance");
        let mut events = Events::with_capacity(1024);

        poll.registry()
            .register(&mut self.listener, SERVER_TOKEN, Interest::READABLE)
            .expect("failed to register TCP listener with Master Poll");

        poll.registry()
            .register(&mut self.udp_listener, UDP_SERVER_TOKEN, Interest::READABLE)
            .expect("failed to register UDP listener with Master Poll");

        loop {
            if let Err(e) = poll.poll(&mut events, None) {
                eprintln!("master poll error: {e}");
                continue;
            }

            for event in events.iter() {
                match event.token() {
                    SERVER_TOKEN => loop {
                        match self.listener.accept() {
                            Ok((stream, _addr)) => {
                                self.dispatch_stream(stream);
                            }
                            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                                break;
                            }
                            Err(e) => {
                                eprintln!("master accept error: {e}");
                                break;
                            }
                        }
                    },
                    UDP_SERVER_TOKEN => {
                        let mut buf = [0u8; 4096];
                        loop {
                            match self.udp_listener.recv_from(&mut buf) {
                                Ok((len, peer_addr)) => {
                                    let mut h3_conn = Http3Connection::new(peer_addr);
                                    if let Ok(requests) = h3_conn.process_datagram(&buf[..len]) {
                                        for (stream_id, req) in requests {
                                            let res = handler::route(&req);
                                            let res_bytes =
                                                Http3Connection::encode_response(stream_id, &res);
                                            let _ = self.udp_listener.send_to(&res_bytes, peer_addr);
                                        }
                                    }
                                }
                                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                                    break;
                                }
                                Err(e) => {
                                    eprintln!("udp recv error: {e}");
                                    break;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn dispatch_stream(&mut self, stream: TcpStream) {
        if self.workers.is_empty() {
            return;
        }

        let worker = &self.workers[self.next_worker];
        if let Ok(()) = worker.sender.send(stream) {
            let _ = worker.waker.wake();
        }

        self.next_worker = (self.next_worker + 1) % self.workers.len();
    }
}