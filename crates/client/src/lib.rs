pub mod audio;
pub mod commands;
pub mod mumble;
pub mod network;
pub mod tls;

pub use speakez;

use std::{collections::VecDeque, sync::Arc};

use audio::DeviceConfig;
use speakez::{
    common::{events::UserSwitchedChannel, ChannelID},
    mumble::control::{
        proto::{self},
        Message as _, MessageBuf,
    },
};

pub use self::commands::Command;

pub mod outgoing {
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(tag = "type", content = "data"))]
    #[derive(Clone, Debug)]
    pub enum Event {
        Speakez(speakez::common::events::Event),
    }
}

#[derive(Debug)]
pub enum Event {
    InputData(Vec<u8>),
    Control(network::events::Message<MessageBuf>),
    Voice(network::events::Message<Vec<u8>>),
    Tick(),
}

#[derive(Debug)]
pub enum Message {
    Command {
        tag: Option<commands::Tag>,
        cmd: Command,
    },
    Event(Event),
}

impl Message {
    // fn event(event: Event) -> Self {
    //     Message::Event { tag: None, event }
    // }

    // fn tagged_event(tag: commands::Tag, event: Event) -> Self {
    //     Message::Event {
    //         tag: Some(tag),
    //         event,
    //     }
    // }
}

pub struct Config {
    pub input: DeviceConfig,
    pub output: DeviceConfig,
    /// Specify the delay between input and output in milliseconds
    pub latency: f32,
}

#[derive(Debug)]
struct Auth {
    username: String,
    password: String,
}

#[derive(Debug)]
enum State {
    NotConnected,
    WaitingForConnection(Auth),
    Handshake {
        auth: Option<Auth>,
        state: speakez::client::handshake::State,
    },
    Connected {
        state: speakez::client::State,
    },
}

impl State {
    fn handle_control<F>(
        &mut self,
        m: MessageBuf,
        mut on_complete: F,
    ) -> Option<impl speakez::mumble::control::Message>
    where
        F: FnMut(&mut speakez::client::State),
    {
        let state = std::mem::replace(self, State::NotConnected);

        let mut msg = None;
        match state {
            State::NotConnected => {
                tracing::debug!("dropping message {:?}, most likely old message: ", m.typ);
                // TODO: ignore for now, when disconnecting look at tagging
                // messages with a lifetime so they can be safely ignored
            }
            State::WaitingForConnection(_) => {
                tracing::debug!("dropping message {:?}, most likely old message: ", m.typ);
            }
            State::Handshake { mut auth, state } => {
                match state.handle_message(m) {
                    speakez::client::handshake::Status::Handshake(mut state) => {
                        if let speakez::mumble::handshake::client::State::ServerVersion(_) =
                            state.state
                        {
                            let auth = auth.take().unwrap();
                            // send auth after getting server version
                            let auth = proto::Authenticate {
                                username: Some(auth.username),
                                password: Some(auth.password),
                                opus: Some(true),
                                ..Default::default()
                            };

                            msg = Some(auth);
                            state.state =
                                speakez::mumble::handshake::client::State::SentAuthenticate;
                        }
                        *self = State::Handshake { auth, state }
                    }
                    speakez::client::handshake::Status::Connected(mut state) => {
                        on_complete(&mut state);
                        *self = State::Connected { state }
                    }
                };
            }
            State::Connected { state } => {
                let msg = speakez::client::Message::Mumble(m);
                let state = speakez::client::handle_message(state, msg);
                *self = State::Connected { state };
            }
        };

        msg
    }
}

// TODO: Generate struct with all optional fields
#[derive(Default)]
pub struct Settings {
    input: Option<String>,
    output: Option<String>,

    input_gain: f32,
    input_monitor: bool,
}

pub type Sender = Arc<dyn Fn(Message) -> Result<(), ()> + Send + Sync>;

#[derive(Debug)]
struct Commands {
    tags: VecDeque<(&'static str, commands::Tag)>,
    messages: Vec<commands::response::Message>,
}

impl Commands {
    fn new() -> Self {
        Self {
            tags: VecDeque::with_capacity(16),
            messages: Vec::with_capacity(16),
        }
    }

    fn push_tag(&mut self, cmd: &'static str, tag: commands::Tag) {
        self.tags.push_back((cmd, tag));
    }

    fn get_next_tag(&mut self, cmd: &'static str) -> Option<commands::Tag> {
        let pos = self
            .tags
            .iter_mut()
            .enumerate()
            .find_map(|(idx, (name, _tag))| if cmd == *name { Some(idx) } else { None })?;
        self.tags.remove(pos).map(|(_, tag)| tag)
    }

    fn find_tag(&mut self, t: commands::Tag) -> Option<commands::Tag> {
        let pos = self
            .tags
            .iter_mut()
            .enumerate()
            .find_map(|(idx, (_name, tag))| if *tag == t { Some(idx) } else { None })?;
        self.tags.remove(pos).map(|(_, tag)| tag)
    }

    fn push_response(&mut self, resp: commands::Response) {
        let msg = match self.get_next_tag(resp.command_name()) {
            Some(tag) => commands::response::Message::ok(tag, resp),
            None => return,
        };
        self.messages.push(msg);
    }

    fn push_error_response(&mut self, tag: commands::Tag, err: String) {
        let msg = commands::response::Message::err(tag, err);
        self.messages.push(msg);
    }
}

fn complete_response(state: &speakez::client::State) -> commands::Response {
    commands::response::Connect {
        session: state.session,
        users: state
            .users
            .values()
            .map(|v| v.to_owned())
            .collect::<Vec<speakez::common::User>>(),
        channels: state
            .channels
            .values()
            .map(|v| v.to_owned())
            .collect::<Vec<speakez::common::Channel>>(),
    }
    .into()
}

pub struct Client {
    sender: Sender,
    settings: Settings,
    state: State,
    commands: Commands,
    network: network::State,
    audio: audio::State,
}

impl Client {
    pub fn new(cfg: rustls::ClientConfig, sender: Sender) -> Self {
        let s = sender.clone();
        let control = move |msg| {
            let event = Event::Control(msg);
            s(Message::Event(event)).unwrap();
        };

        let s = sender.clone();
        let voice = move |msg| {
            let event = Event::Voice(msg);
            s(Message::Event(event)).unwrap();
        };

        let s = sender.clone();
        let input = move |event| {
            let event = match event {
                audio::state::Event::Error(e) => panic!("audio input error: {}", e),
                audio::state::Event::Data(d) => Event::InputData(d),
            };
            s(Message::Event(event)).unwrap();
        };

        let output = move |event| match event {
            audio::state::Event::Error(e) => panic!("audio output error: {}", e),
            audio::state::Event::Data(d) => panic!("output should not be sending data"),
        };

        Self {
            sender,
            settings: Settings::default(),
            state: State::NotConnected,
            commands: Commands::new(),
            audio: audio::State::new(Box::new(input), Box::new(output)),
            network: network::State::new(cfg, Box::new(control), Box::new(voice)),
        }
    }

    fn network_disconnected(&mut self) {
        if self.network.all_disconnected() {
            let resp = commands::Response::Disconnect(commands::response::Disconnect);
            self.commands.push_response(resp);
        }
    }

    fn control_connected(&mut self) {
        self.network.control_connected = true;
        let auth = match std::mem::replace(&mut self.state, State::NotConnected) {
            State::WaitingForConnection(auth) => auth,
            s => panic!("unexpected state during connect, got: {:?}", s),
        };
        self.state = State::Handshake {
            auth: Some(auth),
            state: speakez::client::handshake::State::new(),
        };

        let v = speakez::client::version();
        let cmd = network::Command::Send(v.as_vec());
        self.network.control.send(cmd, None);
        dbg!("connected and sent version");
    }

    fn connect(&mut self, cmd: commands::Connect, tag: Option<commands::Tag>) {
        self.state = State::WaitingForConnection(Auth {
            username: cmd.user,
            password: cmd.pass,
        });

        let network_cmd = network::commands::Connect { addr: cmd.addr };
        let cmd = network::Command::Connect(network_cmd);
        self.network.control.send(cmd, tag);
    }

    fn disconnect(&mut self) {
        if let State::NotConnected = self.state {
            return;
        }

        let cmd = network::Command::Disconnect;
        self.network.control.send(cmd.clone(), None);
        self.network.voice.send(cmd, None);

        self.state = State::NotConnected;
    }

    fn switch_channel(&mut self, to_channel: ChannelID) {
        let state = match self.get_state_mut() {
            Some(s) => s,
            None => return,
        };

        let from_channel = state.get_self().channel;
        let m = UserSwitchedChannel {
            user: state.session,
            from_channel,
            to_channel,
        }
        .into_mumble();

        let cmd = network::Command::Send(m.as_vec());
        self.network.control.send(cmd, None);
    }

    fn handle_command(&mut self, tag: Option<commands::Tag>, cmd: Command) {
        if let Some(tag) = tag {
            self.commands.push_tag(cmd.name(), tag);
        }

        match cmd {
            Command::Connect(c) => {
                self.disconnect();
                self.connect(c, tag);
            }
            Command::Disconnect(..) => self.disconnect(),
            Command::SwitchChannel(cmd) => self.switch_channel(cmd.channel_id),
            Command::SendMessage(send_message) => todo!(),
            Command::MuteMic(mute_mic) => todo!(),
            Command::MonitorMic(monitor_mic) => todo!(),
            Command::SetInputDevice(d) => {
                let cmd = audio::state::Command::SetDevice(d.cfg);
                self.audio.input.send(cmd);
            }
            Command::SetOutuptDevice(d) => {
                let cmd = audio::state::Command::SetDevice(d.cfg);
                self.audio.output.send(cmd);
            }
        }
    }

    fn handle_audio_input(&mut self, encoded_pcm: Vec<u8>) {
        let state = match self.state {
            State::Connected { ref mut state, .. } => state,
            _ => return,
        };

        self.audio.frame_number += 1;
        let msg = speakez::common::events::VoiceMessage {
            frame_number: self.audio.frame_number,
            data: encoded_pcm,
            sender: state.session,
        }
        .into();

        if self.network.tunnel_voice {
            // 1 for the message type
            let length = 1 + speakez::mumble::voice::message_length(&msg);
            let mut buf = vec![0u8; proto::PREFIX_TOTAL_SIZE + length];

            speakez::mumble::control::write_message_header(
                speakez::mumble::control::MessageType::UDPTunnel,
                length,
                &mut buf[..],
            );

            let msg = speakez::mumble::voice::Message::Audio(msg);
            msg.encode(&mut buf[proto::PREFIX_TOTAL_SIZE..]).unwrap();

            let cmd = network::Command::Send(buf);
            self.network.control.send(cmd, None);
        } else {
            // encrypt data
            // send via voice
        }
    }

    fn handle_event(&mut self, e: Event) {
        match e {
            Event::InputData(data) => self.handle_audio_input(data),
            Event::Control(event) => match event.data {
                Ok(event) => match event {
                    network::Event::Connected => {
                        self.control_connected();
                    }
                    network::Event::Disconnected => {
                        self.network.control_connected = false;
                        self.network_disconnected()
                    }
                    network::Event::Data(m) => {
                        let msg = self.state.handle_control(m, |state| {
                            let resp = complete_response(state);
                            // dbg!(&resp);
                            self.commands.push_response(resp);
                        });
                        if let Some(msg) = msg {
                            let cmd = network::Command::Send(msg.as_vec());
                            self.network.control.send(cmd, None);
                        }
                    }
                },
                Err(e) => {
                    let tag = event.tag.and_then(|tag| self.commands.find_tag(tag));
                    if let Some(tag) = tag {
                        self.commands.push_error_response(tag, e.to_string());
                        return;
                    }
                    panic!("network: control error: {e:?}");
                }
            },
            Event::Voice(event) => match event.data {
                Ok(event) => match event {
                    network::Event::Connected => todo!(),
                    network::Event::Disconnected => {
                        self.network.voice_connected = false;
                        self.network_disconnected()
                    }
                    network::Event::Data(_) => {
                        // decrypt data
                    }
                },
                Err(e) => panic!("network: voice error: {e:?}"),
            },
            Event::Tick() => {}
        }

        self.check_for_audio_events();
    }

    fn check_for_audio_events(&mut self) {
        let state = match self.state {
            State::Connected { ref mut state, .. } => state,
            _ => return,
        };

        let events = FilterRemoveIter::new(&mut state.outbox, |event| {
            matches!(event, speakez::common::events::Event::UserSentAudio { .. })
        })
        .map(|event| match event {
            speakez::common::events::Event::UserSentAudio(msg) => msg,
            _ => unreachable!("event should be UserSentAudio as it is matched above"),
        });

        for event in events {
            let cmd = audio::state::Command::PlayOpusAudio(event.data);
            self.audio.output.send(cmd);
        }
    }

    fn get_state_mut(&mut self) -> Option<&mut speakez::client::State> {
        match self.state {
            State::Connected { ref mut state, .. } => Some(state),
            _ => None,
        }
    }

    pub fn handle_message(&mut self, m: Message) {
        match m {
            Message::Command { tag, cmd } => self.handle_command(tag, cmd),
            Message::Event(event) => self.handle_event(event),
        }
    }

    pub fn events(&mut self) -> Option<impl Iterator<Item = speakez::common::events::Event> + '_> {
        self.get_state_mut().map(|s| s.outbox.drain(..))
    }

    pub fn responses(&mut self) -> impl Iterator<Item = commands::response::Message> + '_ {
        self.commands.messages.drain(..)
    }
}

struct FilterRemoveIter<'a, T, F>
where
    F: Fn(&T) -> bool,
{
    vec: &'a mut Vec<T>,
    predicate: F,
}

impl<'a, T, F> FilterRemoveIter<'a, T, F>
where
    F: Fn(&T) -> bool,
{
    fn new(vec: &'a mut Vec<T>, predicate: F) -> Self {
        FilterRemoveIter { vec, predicate }
    }
}

impl<T, F> Iterator for FilterRemoveIter<'_, T, F>
where
    F: Fn(&T) -> bool,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let mut index = 0;
        while index < self.vec.len() {
            if (self.predicate)(&self.vec[index]) {
                let removed_item = self.vec.swap_remove(index);
                return Some(removed_item);
            } else {
                index += 1;
            }
        }
        None
    }
}

pub struct ClientRef {
    sender: Sender,
}

impl From<&Client> for ClientRef {
    fn from(value: &Client) -> Self {
        Self {
            sender: value.sender.clone(),
        }
    }
}

impl ClientRef {
    fn send_command(&self, cmd: impl Into<Command>) {
        let cmd = cmd.into();
        (self.sender)(Message::Command { tag: None, cmd }).unwrap();
    }

    pub fn connect(&self, c: commands::Connect) {
        self.send_command(c);
    }

    pub fn disconnect(&self) {
        self.send_command(commands::Disconnect);
    }

    pub fn input_set_device(&self, config: DeviceConfig) {
        let cmd = commands::SetInput {
            cfg: audio::state::commands::SetDevice { config },
        };
        self.send_command(cmd);
    }

    pub fn input_set_device_state(&self) {
        todo!()
    }

    pub fn output_set_device(&self, config: DeviceConfig) {
        let cmd = commands::SetOutput {
            cfg: audio::state::commands::SetDevice { config },
        };
        self.send_command(cmd);
    }

    pub fn output_set_device_state(&self) {
        todo!()
    }
}
