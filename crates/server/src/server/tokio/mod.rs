mod shutdown;
mod tcp;
mod udp;
mod unix_socket;

use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::net::{TcpListener, UdpSocket, UnixListener};
use tokio::sync::mpsc::{self};
use tokio::sync::{broadcast, oneshot};
use tokio::task::JoinHandle;
use tokio_rustls::TlsAcceptor;

use speakez::mumble;
use speakez::mumble::session::Session;
use speakez::server::state::State;
use speakez::server::{self, state};

use self::shutdown::Shutdown;
use self::udp::UdpListener;

pub enum ActorMessage {
    CreateSession(mpsc::Sender<Vec<u8>>, oneshot::Sender<Option<Session>>),
    Message(server::Message),
}

fn drain_messages(
    mailboxes: &mut HashMap<Session, mpsc::Sender<Vec<u8>>>,
    udp_mailbox: &mut mpsc::Sender<(Vec<u8>, SocketAddr)>,
    s: &mut State,
) {
    let mut to_remove = vec![];

    for msg in s.outbox.drain(..) {
        let sessions: &mut dyn Iterator<Item = (&Session, &mpsc::Sender<Vec<u8>>)> = match msg.dest
        {
            state::OutboxDestination::Session(dest) => match dest {
                server::Destination::All => &mut mailboxes.iter(),
                server::Destination::AllButOne(s) => &mut mailboxes
                    .iter()
                    .filter(move |(session, _mailbox)| s != **session),
                server::Destination::Group(sessions) => &mut mailboxes
                    .iter()
                    .filter(move |(session, _mailbox)| sessions.contains(session)),
                server::Destination::Single(ref session) => {
                    let found = mailboxes
                        .iter()
                        .find(|(s, _mailbox)| *s == session)
                        .unwrap();
                    &mut std::iter::once(found)
                }
            },
            state::OutboxDestination::SocketAddr(addr) => {
                // message is sent unencrypted when given a SocketAddr vs a session.
                udp_mailbox.blocking_send((msg.data, addr)).unwrap();
                continue;
            }
        };

        for (session, mailbox) in sessions {
            match msg.typ {
                state::OutboxType::Control => {
                    if mailbox.blocking_send(msg.data.clone()).is_err() {
                        to_remove.push(*session)
                    }
                }
                state::OutboxType::Voice => {
                    // Handle session not existing anymore
                    let info = match s.session_info.get_mut(session) {
                        Some(s) => s,
                        None => {
                            // Session info only contains clients not in the handshake state
                            continue;
                        }
                    };
                    match info.voice_transport {
                        state::VoiceTransport::Tcp => {
                            let size = mumble::control::proto::PREFIX_TOTAL_SIZE + msg.data.len();
                            let mut data = vec![0u8; size];
                            mumble::control::encode_udp_tunnel(&msg.data, &mut data[..]);

                            if mailbox.blocking_send(data).is_err() {
                                to_remove.push(*session)
                            }
                        }
                        state::VoiceTransport::Udp(addr) => {
                            let mut packet: Vec<u8> = Vec::with_capacity(msg.data.len() + 4);
                            bytes::BufMut::put_slice(&mut packet, &[0, 0, 0, 0]);
                            packet.append(&mut msg.data.clone());
                            let mut b = bytes::BytesMut::from(&packet[..]);
                            info.voice_crypter.encrypt(&mut b);
                            udp_mailbox.blocking_send((b.to_vec(), addr)).unwrap();
                        }
                    }
                }
            }
        }

        for session in to_remove.drain(..) {
            mailboxes.remove(&session);
        }
    }
}

pub fn run(
    mut s: state::State,
    mut recv: mpsc::Receiver<ActorMessage>,
    mut udp_mailbox: mpsc::Sender<(Vec<u8>, SocketAddr)>,
) {
    let mut mailboxes = HashMap::with_capacity(s.config.max_users.into());

    while let Some(message) = recv.blocking_recv() {
        let msg = match message {
            ActorMessage::CreateSession(mailbox, resp) => {
                let session = s.new_session();
                resp.send(session).unwrap();

                if let Some(session) = session {
                    mailboxes.insert(session, mailbox);
                    server::Message::SessionCreated(session)
                } else {
                    todo!("no session available");
                }
            }
            ActorMessage::Message(m) => m,
        };

        let now = Instant::now();
        s = server::handle_message(s, msg, now);
        drain_messages(&mut mailboxes, &mut udp_mailbox, &mut s);
    }
}

pub async fn run_io(
    tcp_listener: TcpListener,
    udp_socket: UdpSocket,
    unix_socket: UnixListener,
    acceptor: TlsAcceptor,
    actor_mailbox: mpsc::Sender<ActorMessage>,
    udp_mailbox: mpsc::Receiver<(Vec<u8>, SocketAddr)>,
    shutdown: impl Future,
) {
    let (notify_shutdown, _) = broadcast::channel(1);
    let (shutdown_complete_tx, mut shutdown_complete_rx) = mpsc::channel(1);

    let shutdown_waiter = shutdown_complete_tx.clone();
    let sender = actor_mailbox.clone();

    let mut unix_listener = unix_socket::UnixListener {
        unix_listener: unix_socket,
        actor_mailbox: sender,
        notify_shutdown: notify_shutdown.clone(),
        shutdown_complete_tx: shutdown_waiter,
    };

    let udp_shutdown = Shutdown::new(notify_shutdown.subscribe());
    let shutdown_waiter = shutdown_complete_tx.clone();
    let sender = actor_mailbox.clone();

    let udp_listener = UdpListener {
        udp_socket: Arc::new(udp_socket),
        mailbox: udp_mailbox,
        sender,
        shutdown: udp_shutdown,
        waiter: shutdown_waiter,
    };

    let ticker_shutdown = Shutdown::new(notify_shutdown.subscribe());
    let shutdown_waiter = shutdown_complete_tx.clone();
    let mailbox = actor_mailbox.clone();
    run_ticker(
        Duration::from_millis(100),
        mailbox,
        ticker_shutdown,
        shutdown_waiter,
    );

    let mut server = tcp::Listener {
        tcp_listener,
        acceptor,
        actor_mailbox,
        notify_shutdown,
        shutdown_complete_tx,
    };

    tokio::select! {
        _ = server.run() => {}
        _ = udp_listener.run() => {},
        _ = unix_listener.run() => {},
        _ = shutdown => {
            tracing::debug!("shutting down signal received");
        }
    }

    tracing::info!("shutting down");

    {
        let tcp::Listener {
            shutdown_complete_tx,
            notify_shutdown,
            ..
        } = server;

        drop(notify_shutdown);
        drop(shutdown_complete_tx);

        tracing::debug!("tokio tcp server shutdown");
    }

    {
        let unix_socket::UnixListener {
            shutdown_complete_tx,
            notify_shutdown,
            ..
        } = unix_listener;

        drop(notify_shutdown);
        drop(shutdown_complete_tx);

        tracing::debug!("tokio unix server shutdown");
    }

    // Wait for all active connections to finish processing.
    let duration = Duration::from_secs(5);
    if tokio::time::timeout(duration, shutdown_complete_rx.recv())
        .await
        .is_err()
    {
        tracing::warn!("shutdown timed out");
    };
}

/// Spawn a new tokio task that ticks every duration.
fn run_ticker(
    duration: Duration,
    mailbox: mpsc::Sender<ActorMessage>,
    mut shutdown: Shutdown,
    waiter: mpsc::Sender<()>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(duration);
        loop {
            tokio::select! {
                _ = ticker.tick() => {},
                _ = shutdown.recv() => break,
            };

            mailbox
                .send(ActorMessage::Message(server::Message::Tick))
                .await
                .unwrap();
        }

        drop(waiter);
    })
}
