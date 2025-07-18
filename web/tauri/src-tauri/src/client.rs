use speakez_client::{
    audio::state::commands as audio_commands,
    commands::{self, response},
    speakez::common::ChannelID,
    Command, Message,
};
use tokio::sync::broadcast;

async fn wait_for_message<M, T, F>(
    mut receiver: broadcast::Receiver<M>,
    predicate: F,
) -> Result<T, broadcast::error::RecvError>
where
    M: Clone,
    F: Fn(M) -> Option<T>,
{
    loop {
        match receiver.recv().await {
            Ok(msg) => {
                if let Some(msg) = predicate(msg) {
                    return Ok(msg);
                }
            }
            Err(e) => return Err(e),
        }
    }
}

fn random_tag() -> commands::Tag {
    let n = rand::random::<u32>();
    commands::Tag(n)
}

pub struct ClientRef {
    sender: speakez_client::Sender,
    tx: broadcast::Sender<commands::response::Message>,
}

impl ClientRef {
    pub fn new(
        sender: speakez_client::Sender,
        tx: broadcast::Sender<commands::response::Message>,
    ) -> Self {
        Self { tx, sender }
    }

    fn send_command(&self, cmd: Command) {
        (self.sender)(Message::Command { tag: None, cmd }).unwrap();
    }

    async fn send_with_resp<C, R>(&self, cmd: C) -> Result<R, String>
    where
        C: commands::Cmd,
        R: response::ResponseFor<Command = C>,
    {
        let rx = self.tx.subscribe();
        let tag = random_tag();

        let msg = wait_for_message(rx, |msg| {
            if msg.tag != tag {
                return None;
            }
            Some(msg.resp)
        });

        let cmd_name = cmd.name();
        (self.sender)(Message::Command {
            tag: Some(tag),
            cmd: cmd.into(),
        })
        .unwrap();

        msg.await
            .map_err(|e: broadcast::error::RecvError| e.to_string())?
            .and_then(|resp| {
                resp.try_into()
                    .map_err(|_| format!("invalid type received as a response for {cmd_name}"))
            })
    }

    pub async fn connect(&self, c: commands::Connect) -> Result<response::Connect, String> {
        self.send_with_resp(c).await
    }

    pub async fn disconnect(&self) -> Result<response::Disconnect, String> {
        self.send_with_resp(commands::Disconnect).await
    }

    pub fn switch_channel(&self, channel_id: ChannelID) {
        self.send_command(commands::SwitchChannel { channel_id }.into())
    }

    pub fn input_set_device(&self, cfg: speakez_client::audio::DeviceConfig) {
        let cfg = audio_commands::SetDevice { config: cfg };
        self.send_command(commands::SetInput { cfg }.into())
    }

    pub fn output_set_device(&self, cfg: speakez_client::audio::DeviceConfig) {
        let cfg = audio_commands::SetDevice { config: cfg };
        self.send_command(commands::SetOutput { cfg }.into())
    }

    pub fn input_mute(&self, value: bool) {
        // self.send_command(cmd);
    }

    pub fn input_monitor(&self, value: bool) {
        // self.send_command(cmd);
    }
}
