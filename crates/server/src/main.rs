use std::fs::File;
use std::io::{self, BufReader};
use std::num::NonZeroI32;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use speakez::common::{Channel, ChannelID};
use speakez::server::state::{State, VoiceCrypter};

use rustls_pemfile::{certs, private_key};
use speakez_server::server;
use speakez_server::server::tokio::ActorMessage;
use tokio::net::{TcpListener, UdpSocket, UnixListener};
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio_rustls::TlsAcceptor;

fn main() {
    init_subscriber();

    tracing::info!("speakez starting");
    run();
    tracing::info!("speakez shutting down");
}

fn init_subscriber() {
    use tracing::Level;
    use tracing_subscriber::FmtSubscriber;

    // NOTE: This feature adds 1.0 MBs to the binary size
    // let filter = tracing_subscriber::EnvFilter::from_default_env();

    let subscriber = FmtSubscriber::builder()
        // .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}

fn run() {
    let acceptor = new_acceptor();

    let (sender, reciever) = tokio::sync::mpsc::channel::<ActorMessage>(100);
    let (udp_sender, udp_reciever) = tokio::sync::mpsc::channel(100);

    let state_thread = std::thread::spawn(move || {
        let state = load_state();
        server::tokio::run(state, reciever, udp_sender);
        tracing::info!("server state shutdown");
    });

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();

    rt.block_on(async {
        let tcp_listener = TcpListener::bind("0.0.0.0:64738").await.unwrap();
        let udp_socket = UdpSocket::bind("0.0.0.0:64738").await.unwrap();

        let unix_socket = {
            let socket_path = "/tmp/speakez.sock";
            if Path::new(socket_path).exists() {
                std::fs::remove_file(socket_path).unwrap();
            }
            UnixListener::bind(socket_path).unwrap()
        };

        server::tokio::run_io(
            tcp_listener,
            udp_socket,
            unix_socket,
            acceptor,
            sender,
            udp_reciever,
            tokio::signal::ctrl_c(),
        )
        .await;
        tracing::info!("tokio server shutdown");
    });

    state_thread.join().unwrap();
}

fn new_crypter() -> Box<dyn VoiceCrypter> {
    use speakez_server::mumble::crypt;

    let mut key = [0u8; crypt::KEY_SIZE];
    crypt::fill(&mut key).unwrap();
    Box::new(crypt::CryptState::new_from_key(key))
}

fn load_state() -> State {
    let mut s = State::new(100, new_crypter);

    let root = ChannelID::new(0);

    s.new_channel(Channel::new(
        root,
        "TestChannel".to_string(),
        "Description".to_string(),
        false,
        None,
    ));
    s.new_channel(Channel {
        id: ChannelID::new(1),
        name: "SubChannel".to_string(),
        description: "Description".to_string(),
        temporary: false,
        max_users: None,
        position: Some(NonZeroI32::new(-1).unwrap()),
        parent: Some(root),
    });
    s
}

fn load_certs(path: &Path) -> io::Result<Vec<CertificateDer<'static>>> {
    certs(&mut BufReader::new(File::open(path)?)).collect()
}

fn load_keys(path: &Path) -> io::Result<PrivateKeyDer<'static>> {
    Ok(private_key(&mut BufReader::new(File::open(path)?))?.unwrap())
}

fn new_acceptor() -> TlsAcceptor {
    let certs = load_certs(&PathBuf::from("./keys/cert.pem")).unwrap();
    let key = load_keys(&PathBuf::from("./keys/key.pem")).unwrap();
    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))
        .unwrap();
    TlsAcceptor::from(Arc::new(config))
}
