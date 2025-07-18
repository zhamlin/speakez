use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use speakez::{mumble, server};

use super::shutdown::Shutdown;
use super::ActorMessage;

pub(crate) struct UdpListener {
    pub udp_socket: Arc<UdpSocket>,
    pub sender: mpsc::Sender<ActorMessage>,
    pub mailbox: mpsc::Receiver<(Vec<u8>, SocketAddr)>,
    pub shutdown: Shutdown,
    pub waiter: mpsc::Sender<()>,
}

impl UdpListener {
    pub async fn run(mut self) {
        let reader = self.udp_socket.clone();

        let mut writer = tokio::spawn(async move {
            loop {
                let (data, to) = tokio::select! {
                    // _ = self.shutdown.recv() => break,
                    v = self.mailbox.recv() => match v {
                        Some(v) => v,
                        None => break,
                    },
                };

                self.udp_socket.send_to(&data, to).await.unwrap();
            }
            tracing::debug!("tokio udp writer server shutdown");
        });

        let mailbox = self.sender.clone();
        let mut reader = tokio::spawn(async move {
            let mut buf = vec![0u8; mumble::voice::MAX_UDP_PACKET_SIZE];

            loop {
                let (size, from) = tokio::select! {
                    _ = self.shutdown.recv() => break,
                    res = reader.recv_from(&mut buf) => match res {
                        Ok(v) => v,
                        Err(e) => todo!("error: {}",e),
                    }
                };

                let msg = server::Message::UDP(from, buf[..size].to_vec());
                mailbox.send(ActorMessage::Message(msg)).await.unwrap();
            }

            drop(self.waiter);
            tracing::debug!("tokio udp reader server shutdown");
        });

        tokio::select! {
            _ = &mut reader => false,
            _ = &mut writer => false,
        };

        reader.abort();
        writer.abort();

        tracing::debug!("tokio udp server shutdown");
    }
}
