use std::io;

use tokio::sync::{broadcast, mpsc};
use tracing::Instrument;

use super::shutdown::Shutdown;
use super::tcp::Handler;
use super::ActorMessage;

pub struct UnixListener {
    pub unix_listener: tokio::net::UnixListener,
    pub actor_mailbox: mpsc::Sender<ActorMessage>,

    /// See tcp::Listener for more information
    pub notify_shutdown: broadcast::Sender<()>,
    pub shutdown_complete_tx: mpsc::Sender<()>,
}

impl UnixListener {
    pub async fn run(&mut self) -> Result<(), ()> {
        while let Ok((stream, addr)) = self.unix_listener.accept().await {
            if self.actor_mailbox.is_closed() {
                return Ok(());
            }

            tracing::debug!("accepted unix stream connection");

            let cloned_mailbox = self.actor_mailbox.clone();
            let shutdown = Shutdown::new(self.notify_shutdown.subscribe());
            let _shutdown_complete = self.shutdown_complete_tx.clone();

            tokio::spawn(async move {
                let (reader, writer) = tokio::io::split(stream);
                let (sender, mailbox) = mpsc::channel(100);

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

                let span = tracing::info_span!("unix socket connection", ?addr);
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
