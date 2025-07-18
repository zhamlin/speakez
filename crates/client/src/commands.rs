pub mod response {
    use super::Cmd;

    pub trait ResponseFor: std::convert::TryFrom<Response> {
        type Command: Cmd;

        fn cmd(&self) -> &'static str {
            Self::Command::NAME
        }
    }

    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(tag = "type", content = "data"))]
    #[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
    #[derive(Clone, Debug)]
    pub enum Response {
        Connect(Connect),
        Disconnect(Disconnect),
    }

    impl Response {
        pub fn command_name(&self) -> &'static str {
            match self {
                Response::Connect(r) => r.cmd(),
                Response::Disconnect(r) => r.cmd(),
            }
        }
    }

    // region:Response::from
    impl From<Connect> for Response {
        fn from(val: Connect) -> Self {
            Response::Connect(val)
        }
    }

    impl TryFrom<Response> for Connect {
        type Error = ();

        fn try_from(value: Response) -> Result<Self, Self::Error> {
            match value {
                Response::Connect(val) => Ok(val),
                _ => Err(()),
            }
        }
    }

    impl From<Disconnect> for Response {
        fn from(val: Disconnect) -> Self {
            Response::Disconnect(val)
        }
    }

    impl TryFrom<Response> for Disconnect {
        type Error = ();

        fn try_from(value: Response) -> Result<Self, Self::Error> {
            match value {
                Response::Disconnect(val) => Ok(val),
                _ => Err(()),
            }
        }
    }
    // endregion:Response::from

    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Clone, Debug)]
    pub struct Message {
        pub tag: super::Tag,
        pub resp: Result<Response, String>,
    }

    impl Message {
        pub fn ok(tag: super::Tag, resp: Response) -> Self {
            Self {
                tag,
                resp: Ok(resp),
            }
        }

        pub fn err(tag: super::Tag, err: String) -> Self {
            Self {
                tag,
                resp: Err(err),
            }
        }
    }

    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
    #[derive(Clone, Debug)]
    pub struct Connect {
        pub session: speakez::mumble::session::Session,
        pub users: Vec<speakez::common::User>,
        pub channels: Vec<speakez::common::Channel>,
    }

    impl ResponseFor for Connect {
        type Command = super::Connect;
    }

    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
    #[derive(Clone, Debug)]
    pub struct Disconnect;

    impl ResponseFor for Disconnect {
        type Command = super::Disconnect;
    }
}

use crate::audio;
pub use response::Response;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Tag(pub u32);

#[derive(Debug)]
pub struct Connect {
    pub addr: String,
    pub user: String,
    pub pass: String,
}

impl Cmd for Connect {
    const NAME: &'static str = "connect";
}

#[derive(Debug)]
pub struct Disconnect;

impl Cmd for Disconnect {
    const NAME: &'static str = "disconnect";
}

#[derive(Debug)]
pub struct SwitchChannel {
    pub channel_id: speakez::common::ChannelID,
}

impl Cmd for SwitchChannel {
    const NAME: &'static str = "switch-channel";
}

#[derive(Debug)]
pub struct SendMessage {}

#[derive(Debug)]
pub struct ChangeInput {
    volume: Option<u32>,
    device: Option<String>,
}

#[derive(Debug)]
pub struct ChangeInputDevice {
    device: String,
}

#[derive(Debug)]
pub struct ChangeInputVolume {
    level: u32,
}

#[derive(Debug)]
pub struct ChangeOutput {}

#[derive(Debug)]
pub struct MuteMic {
    pub mute: bool,
}

#[derive(Debug)]
pub struct MonitorMic {
    monitor: bool,
}

#[derive(Debug)]
pub struct SetInput {
    pub cfg: audio::state::commands::SetDevice,
}

#[derive(Debug)]
pub struct SetOutput {
    pub cfg: audio::state::commands::SetDevice,
}

pub trait Cmd: Into<Command> {
    const NAME: &'static str;

    fn name(&self) -> &'static str {
        Self::NAME
    }
}

#[derive(Debug)]
pub enum Command {
    Connect(Connect),
    Disconnect(Disconnect),
    SwitchChannel(SwitchChannel),
    SendMessage(SendMessage),
    MuteMic(MuteMic),
    MonitorMic(MonitorMic),
    SetInputDevice(SetInput),
    SetOutuptDevice(SetOutput),
}

impl Command {
    pub fn name(&self) -> &'static str {
        match self {
            Command::Connect(cmd) => cmd.name(),
            Command::Disconnect(cmd) => cmd.name(),
            Command::SwitchChannel(cmd) => cmd.name(),
            _ => todo!(),
        }
    }
}

// region:Command::from
impl From<Connect> for Command {
    fn from(val: Connect) -> Self {
        Command::Connect(val)
    }
}

impl From<Disconnect> for Command {
    fn from(val: Disconnect) -> Self {
        Command::Disconnect(val)
    }
}

impl From<SwitchChannel> for Command {
    fn from(val: SwitchChannel) -> Self {
        Command::SwitchChannel(val)
    }
}

impl From<SendMessage> for Command {
    fn from(val: SendMessage) -> Self {
        Command::SendMessage(val)
    }
}

impl From<MuteMic> for Command {
    fn from(val: MuteMic) -> Self {
        Command::MuteMic(val)
    }
}

impl From<MonitorMic> for Command {
    fn from(val: MonitorMic) -> Self {
        Command::MonitorMic(val)
    }
}

impl From<SetInput> for Command {
    fn from(val: SetInput) -> Self {
        Command::SetInputDevice(val)
    }
}

impl From<SetOutput> for Command {
    fn from(val: SetOutput) -> Self {
        Command::SetOutuptDevice(val)
    }
}
// endregion:Command::from

#[test]
fn sourcegen_from_code() {
    use gen::enums::RegionGenerator;
    use std::fmt::Write;

    fn prefix_non_empty_lines<'a>(lines: impl Iterator<Item = &'a str>, prefix: &str) -> String {
        lines
            .map(|line| {
                if line.trim().is_empty() {
                    line.to_string()
                } else {
                    format!("{prefix}{line}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    let generators = vec![RegionGenerator::new(
        "Response::from",
        |variants, indent, _name| {
            variants.iter().fold(String::new(), |mut output, v| {
                let template = format!(
                    r#"
impl From<{0}> for Response {{
    fn from(val: {0}) -> Self {{
        Response::{0}(val)
    }}
}}

impl TryFrom<Response> for {0} {{
    type Error = ();

    fn try_from(value: Response) -> Result<Self, Self::Error> {{
        match value {{
            Response::{0}(val) => Ok(val),
            _ => Err(()),
        }}
    }}
}}


"#,
                    v.name
                );
                let code = prefix_non_empty_lines(template.lines().skip(1), indent);
                output.write_str(&code).unwrap();
                output
            })
        },
    )];

    let file_path = std::path::Path::new(file!().strip_prefix("crates/client/").unwrap());

    gen::enums::sourcegen_from_code(file_path, "Response", 0, &generators);

    let generators = vec![RegionGenerator::new(
        "Command::from",
        |variants, indent, _name| {
            variants.iter().fold(String::new(), |mut output, v| {
                let name = v.typ.name().unwrap_or(&v.name);
                let template = format!(
                    r#"
impl From<{0}> for Command {{
    fn from(val: {0}) -> Self {{
        Command::{1}(val)
    }}
}}


"#,
                    name, v.name
                );
                let code = prefix_non_empty_lines(template.lines().skip(1), indent);
                output.write_str(&code).unwrap();
                output
            })
        },
    )];

    gen::enums::sourcegen_from_code(file_path, "Command", 0, &generators);
}
