use std::io;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_rustls::TlsAcceptor;
use tracing::{Instrument, Span};

use speakez::{mumble, server};

use super::shutdown::Shutdown;
use super::ActorMessage;

/// Handles TCP connections.
pub struct Listener {
    pub tcp_listener: TcpListener,

    pub acceptor: TlsAcceptor,
    pub actor_mailbox: mpsc::Sender<ActorMessage>,

    /// Broadcasts a shutdown signal to all active connections.
    pub notify_shutdown: broadcast::Sender<()>,

    /// Used as part of the graceful shutdown process to wait for client
    /// connections to complete processing.
    pub shutdown_complete_tx: mpsc::Sender<()>,
}

impl Listener {
    pub async fn run(&mut self) -> Result<(), ()> {
        while let Ok((stream, addr)) = self.tcp_listener.accept().await {
            // TODO: remove once tokio thread can be notified of main thread stopping
            if self.actor_mailbox.is_closed() {
                return Ok(());
            }

            let acceptor = self.acceptor.clone();
            let cloned_mailbox = self.actor_mailbox.clone();
            let shutdown = Shutdown::new(self.notify_shutdown.subscribe());
            let _shutdown_complete = self.shutdown_complete_tx.clone();

            tokio::spawn(async move {
                let stream = match acceptor.accept(stream).await {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!("failed to accept TLS stream: {}", e);
                        return;
                    }
                };

                let (reader, writer) = tokio::io::split(stream);
                let (sender, mailbox) = mpsc::channel(20);

                let handler = Handler {
                    reader,
                    writer,
                    sender,
                    mailbox,
                    actor_mailbox: cloned_mailbox,

                    // Receive shutdown notifications.
                    shutdown,

                    // Notifies the receiver half once all clones are
                    // dropped.
                    _shutdown_complete,
                };

                let span = tracing::info_span!("connection", ?addr);
                match handler.run().instrument(span).await {
                    Ok(_) => {}
                    Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => {
                        dbg!("EOF, client probably closed the connection");
                    }
                    Err(err) => {
                        tracing::error!("handle_conn error: {}", err);
                    }
                };
            });
        }

        Ok(())
    }
}

pub struct Handler<R, W> {
    pub reader: R,
    pub writer: W,
    pub actor_mailbox: mpsc::Sender<ActorMessage>,

    pub mailbox: mpsc::Receiver<Vec<u8>>,
    pub sender: mpsc::Sender<Vec<u8>>,
    /// Listen for shutdown notifications.
    ///
    /// A wrapper around the `broadcast::Receiver` paired with the sender in
    /// `Listener`. The connection handler processes requests from the
    /// connection until the peer disconnects **or** a shutdown notification is
    /// received from `shutdown`. In the latter case, any in-flight work being
    /// processed for the peer is continued until it reaches a safe state, at
    /// which point the connection is terminated.
    pub shutdown: Shutdown,

    /// Not used directly. Instead, when `Handler` is dropped the sender is closed.
    pub _shutdown_complete: mpsc::Sender<()>,
}

impl<R, W> Handler<R, W>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    pub async fn run(mut self) -> io::Result<()> {
        let (sender, reciever) = oneshot::channel();
        self.actor_mailbox
            .send(ActorMessage::CreateSession(self.sender, sender))
            .await
            .unwrap();
        let session = reciever
            .await
            .unwrap()
            .expect("a session should be created once a handler has been created");

        let actor_mailbox = self.actor_mailbox.clone();

        let mut read_task = tokio::spawn(
            async move {
                let mut read_buf = vec![0u8; 4096];

                while let Ok((typ, size)) = read_message(&mut read_buf, &mut self.reader).await {
                    let msg = mumble::control::MessageBuf {
                        typ,
                        data: read_buf[..size].to_vec(),
                    };
                    tracing::info!("got message: {:?}", typ);
                    let msg = server::Message::Mumble(session, msg);

                    self.actor_mailbox
                        .send(ActorMessage::Message(msg))
                        .await
                        .unwrap();
                }
                self.actor_mailbox
            }
            .instrument(Span::current()),
        );

        let mut write_task = tokio::spawn(
            async move {
                // TODO: return buf?
                // TODO: send failure to send back to state thread?
                while let Some(msg) = self.mailbox.recv().await {
                    self.writer.write_all(&msg).await.unwrap();
                    self.writer.flush().await.unwrap();
                }
                self.writer.shutdown().await.unwrap();
            }
            .instrument(Span::current()),
        );

        let is_server_shutdown = tokio::select! {
            _ = self.shutdown.recv() => true,
            _ = &mut read_task => false,
            _ = &mut write_task => false,
        };

        read_task.abort();
        write_task.abort();

        if !is_server_shutdown {
            let msg = server::Message::SessionDisconnect(session);
            _ = actor_mailbox.send(ActorMessage::Message(msg)).await;
        }

        tracing::info!("handler shutting down");

        Ok(())
    }
}

pub async fn read_message<T>(
    buf: &mut [u8],
    mut reader: T,
) -> std::io::Result<(mumble::control::MessageType, usize)>
where
    T: AsyncRead + Unpin,
{
    let prefix = &mut buf[..mumble::control::proto::PREFIX_TOTAL_SIZE];
    debug_assert_eq!(prefix.len(), mumble::control::proto::PREFIX_TOTAL_SIZE);

    reader.read_exact(prefix).await?;
    let (typ, size) = mumble::control::parse_prefix(prefix);

    let buf_size = mumble::control::proto::PREFIX_TOTAL_SIZE + size;
    let msg_body = &mut buf[mumble::control::proto::PREFIX_TOTAL_SIZE..buf_size];
    debug_assert_eq!(msg_body.len(), size);

    reader.read_exact(msg_body).await.unwrap();

    Ok((typ, buf_size))
}

// TODO: Add tests for handler
