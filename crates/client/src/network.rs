use std::{
    fmt,
    io::{self, BufReader, BufWriter, ErrorKind, Read, Write},
    net::{AddrParseError, TcpStream},
    sync::{mpsc, Arc},
    time::Duration,
};

pub use commands::Command;
pub use events::Event;
use speakez::mumble::control::MessageBuf;

use crate::commands::Tag;

#[derive(Debug)]
pub struct Message<D> {
    pub tag: Option<Tag>,
    pub data: D,
}

pub mod commands {
    pub type Message = super::Message<Command>;

    #[derive(Clone, Debug)]
    pub struct Connect {
        pub addr: String,
    }

    #[derive(Clone, Debug)]
    pub enum Command {
        Connect(Connect),
        Disconnect,
        Send(Vec<u8>),
    }
}

pub mod events {
    pub type Message<D> = super::Message<Result<Event<D>, super::Error>>;

    #[derive(Debug, PartialEq, Eq)]
    pub enum Event<D> {
        Connected,
        Disconnected,
        Data(D),
    }
}

#[derive(Debug)]
pub enum Error {
    IO(std::io::Error),
    AddrParse(AddrParseError),
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::IO(err) => write!(f, "IO error: {err}"),
            Error::AddrParse(err) => write!(f, "Parse error: {err}"),
            Error::Other(err) => write!(f, "error: {err}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.source()
    }
}

pub type Sender<D> = Box<dyn FnMut(events::Message<D>) + Send>;
pub type ControlSender = Sender<MessageBuf>;
pub type VoiceSender = Sender<Vec<u8>>;

pub struct Handler {
    handle: std::thread::JoinHandle<()>,
    sender: mpsc::Sender<commands::Message>,
}

impl Handler {
    pub fn send(&self, cmd: Command, tag: Option<Tag>) {
        self.sender.send(Message { tag, data: cmd }).unwrap()
    }
}

pub struct State {
    pub control: Handler,
    pub voice: Handler,
    pub tunnel_voice: bool,

    pub control_connected: bool,
    pub voice_connected: bool,
}

impl State {
    pub fn new(cfg: rustls::ClientConfig, control: ControlSender, voice: VoiceSender) -> Self {
        let (sender, receiver) = mpsc::channel();
        let handle = std::thread::Builder::new()
            .name("control_network".to_string())
            .spawn(|| {
                let mut r = TcpReceiver::new(cfg, control);
                r.run(receiver);
            })
            .unwrap();
        let control = Handler { handle, sender };

        let (sender, receiver) = mpsc::channel();
        let handle = std::thread::Builder::new()
            .name("voice_network".to_string())
            .spawn(|| {
                udp_receiver_thread(receiver, voice);
            })
            .unwrap();
        let voice = Handler { handle, sender };

        Self {
            control,
            voice,
            tunnel_voice: true,
            control_connected: false,
            voice_connected: false,
        }
    }

    pub fn all_disconnected(&self) -> bool {
        !self.voice_connected && !self.control_connected
    }
}

fn udp_receiver_thread(receiver: mpsc::Receiver<commands::Message>, mut sender: VoiceSender) {
    let mut socket: Option<std::net::UdpSocket> = None;
    let mut buffer = [0; 1024];
    let timeout = Duration::from_millis(5);

    loop {
        let mut should_sleep = true;

        match receiver.try_recv() {
            Ok(commands::Message { tag, data: cmd }) => {
                should_sleep = false;
                match cmd {
                    Command::Connect(commands::Connect { addr }) => {
                        match std::net::UdpSocket::bind(addr) {
                            Ok(sock) => {
                                sock.set_read_timeout(Some(timeout)).unwrap();
                                socket = Some(sock);
                                let msg = events::Message {
                                    tag,
                                    data: Ok(Event::Connected),
                                };
                                (sender)(msg);
                            }
                            Err(e) => {
                                let msg = events::Message {
                                    tag,
                                    data: Err(Error::IO(e)),
                                };
                                (sender)(msg);
                            }
                        };
                    }
                    Command::Disconnect => {
                        let had_socket = socket.is_some();
                        socket = None;

                        if had_socket {
                            let msg = events::Message {
                                tag,
                                data: Ok(Event::Disconnected),
                            };
                            (sender)(msg);
                        }
                    }
                    Command::Send(data) => {
                        if let Some(ref sock) = socket {
                            match sock.send(&data) {
                                Ok(_) => todo!("check for all data being written"),
                                Err(e) => {
                                    let msg = events::Message {
                                        tag,
                                        data: Err(Error::IO(e)),
                                    };
                                    (sender)(msg);
                                }
                            }
                        }
                    }
                }
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => return,
        }

        // check for data on the socket
        if let Some(ref sock) = socket {
            // read has a timeout set on it, no need to sleep if calling this func
            // TODO: handle errors: ignore timeout
            if let Ok((size, _)) = sock.recv_from(&mut buffer) {
                let data = buffer[..size].to_vec();
                let msg = events::Message {
                    tag: None,
                    data: Ok(Event::Data(data)),
                };
                (sender)(msg)
            }
        } else if should_sleep {
            std::thread::sleep(timeout);
        }
    }
}

struct BufReaderWriter<R>(BufReader<R>);

impl<R> From<BufReader<R>> for BufReaderWriter<R> {
    fn from(r: BufReader<R>) -> Self {
        BufReaderWriter(r)
    }
}

impl<R> std::io::Write for BufReaderWriter<R>
where
    R: std::io::Write,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.get_mut().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.get_mut().flush()
    }
}

struct BufferedTlsStream {
    inner: BufWriter<BufReaderWriter<rustls::StreamOwned<rustls::ClientConnection, TcpStream>>>,
}

impl std::io::Read for BufferedTlsStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.get_mut().0.read(buf)
    }
}

impl std::io::Write for BufferedTlsStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

impl BufferedTlsStream {
    fn new(tls_stream: rustls::StreamOwned<rustls::ClientConnection, TcpStream>) -> Self {
        let buffered = BufReader::new(tls_stream);
        let buffered = BufWriter::new(buffered.into());

        BufferedTlsStream { inner: buffered }
    }
}

struct TcpReceiver {
    cfg: Arc<rustls::ClientConfig>,
    sender: ControlSender,
    conn: Option<BufferedTlsStream>,
    buffer: Vec<u8>,
    connect_timeout: Duration,
    read_timeout: Duration,
}

fn retry_would_block<F, T>(mut f: F) -> io::Result<T>
where
    F: FnMut() -> io::Result<T>,
{
    let mut attempts = 0;
    loop {
        match f() {
            Ok(value) => return Ok(value),
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                attempts += 1;

                if attempts >= 25 {
                    return Err(io::Error::new(
                        ErrorKind::WouldBlock,
                        "Operation would block after 25 attempts",
                    ));
                }

                dbg!("wouldblock, sleeping", attempts);
                std::thread::sleep(std::time::Duration::from_millis(1));
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}

impl TcpReceiver {
    pub fn new(cfg: rustls::ClientConfig, sender: ControlSender) -> Self {
        Self {
            cfg: Arc::new(cfg),
            sender,
            conn: None,
            buffer: vec![0; 4096],
            connect_timeout: Duration::from_millis(200),
            read_timeout: Duration::from_millis(5),
        }
    }

    pub fn run(&mut self, receiver: mpsc::Receiver<commands::Message>) {
        loop {
            let mut should_sleep = true;

            match receiver.try_recv() {
                Ok(commands::Message { tag, data: cmd }) => {
                    should_sleep = false;
                    self.handle_command(tag, cmd);
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => return,
            }

            if self.conn.is_some() {
                should_sleep = false;
                self.handle_socket_data();
            }

            if should_sleep {
                std::thread::sleep(self.read_timeout);
            }
        }
    }

    fn handle_command(&mut self, tag: Option<Tag>, cmd: Command) {
        match cmd {
            Command::Connect(commands::Connect { addr }) => self.handle_connect(tag, addr),
            Command::Disconnect => self.handle_disconnect(tag),
            Command::Send(data) => self.handle_send(tag, data),
        }
    }

    fn handle_connect(&mut self, tag: Option<Tag>, addr: String) {
        let parsed_addr = match addr.parse().map_err(Error::AddrParse) {
            Ok(addr) => addr,
            Err(e) => {
                self.send_error(tag, e);
                return;
            }
        };

        let sock = match TcpStream::connect_timeout(&parsed_addr, self.connect_timeout)
            .map_err(Error::IO)
        {
            Ok(sock) => sock,
            Err(e) => {
                self.send_error(tag, e);
                return;
            }
        };

        if let Err(e) = sock.set_nonblocking(false).map_err(Error::IO) {
            self.send_error(tag, e);
            return;
        }

        // NOTE: when setting read_timeout on macOS/iOS it _appears_ to affect the write timeout.
        if let Err(e) = sock
            .set_read_timeout(Some(self.read_timeout))
            .map_err(Error::IO)
        {
            self.send_error(tag, e);
            return;
        }

        if let Err(e) = sock.set_write_timeout(None).map_err(Error::IO) {
            self.send_error(tag, e);
            return;
        }

        let server_name = parsed_addr.ip().into();
        let tls_conn = match rustls::ClientConnection::new(self.cfg.clone(), server_name) {
            Ok(c) => c,
            Err(e) => {
                self.send_error(tag, Error::Other(e.to_string()));
                return;
            }
        };

        let stream = rustls::StreamOwned::new(tls_conn, sock);
        self.conn = Some(BufferedTlsStream::new(stream));
        self.send_msg(tag, Event::Connected);
    }

    fn handle_disconnect(&mut self, tag: Option<Tag>) {
        self.conn = None;
        self.send_msg(tag, Event::Disconnected);
    }

    fn handle_send(&mut self, tag: Option<Tag>, data: Vec<u8>) {
        let Some(c) = &mut self.conn else { return };

        if let Err(e) = retry_would_block(|| c.write_all(&data)).map_err(Error::IO) {
            tracing::info!("faile to write_all");
            self.send_error(tag, e);
            self.handle_disconnect(None);
            return;
        }

        if let Err(e) = retry_would_block(|| c.flush()).map_err(Error::IO) {
            tracing::info!("faile to flush");
            self.send_error(tag, e);
            self.handle_disconnect(None);
        }
    }

    fn handle_socket_data(&mut self) {
        let Some(conn) = &mut self.conn else { return };
        use speakez::mumble::control::proto::PREFIX_TOTAL_SIZE;

        // TODO: handle partial reads

        let header = &mut self.buffer[..PREFIX_TOTAL_SIZE];
        let read_data = match conn.read_exact(header) {
            Ok(_) => true,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => false,
            Err(e) => {
                self.send_error(None, Error::IO(e));
                return;
            }
        };

        if !read_data {
            return;
        }

        let (typ, size) = speakez::mumble::control::parse_prefix(header);
        let total_size = PREFIX_TOTAL_SIZE + size;

        // read the body
        let remaining = &mut self.buffer[PREFIX_TOTAL_SIZE..total_size];
        match conn.read_exact(remaining) {
            Ok(_) => {
                let data = self.buffer[..total_size].to_vec();
                let msg_buf = MessageBuf { typ, data };
                self.send_msg(None, Event::Data(msg_buf));
            }
            Err(e) => {
                self.conn = None;
                self.send_error(None, Error::IO(e));
            }
        }
    }

    fn send_msg(&mut self, tag: Option<Tag>, e: Event<MessageBuf>) {
        let msg = events::Message { tag, data: Ok(e) };
        (self.sender)(msg);
    }

    fn send_error(&mut self, tag: Option<Tag>, e: Error) {
        let msg = events::Message { tag, data: Err(e) };
        (self.sender)(msg);
    }
}
