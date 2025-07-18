pub mod commands {
    use crate::audio;

    #[derive(Debug)]
    pub struct SetDevice {
        pub config: audio::DeviceConfig,
    }

    #[derive(Debug)]
    pub enum Command {
        SetDevice(SetDevice),
        Pause,
        Play,
        PlayOpusAudio(Vec<u8>),
    }
}

pub mod events {
    #[derive(Debug, PartialEq, Eq)]
    pub enum Event {
        Error(String),
        Data(Vec<u8>),
    }
}

use std::sync::mpsc;

use super::Stream;
use crate::audio;

pub use commands::Command;
use commands::SetDevice;
pub use events::Event;

pub struct Handler {
    handle: std::thread::JoinHandle<()>,
    sender: mpsc::Sender<Command>,
}

impl Handler {
    pub fn new(handle: std::thread::JoinHandle<()>, sender: mpsc::Sender<Command>) -> Self {
        Self { handle, sender }
    }

    pub fn send(&self, cmd: Command) {
        self.sender.send(cmd).unwrap();
    }
}

pub struct State {
    pub input: Handler,
    pub output: Handler,
    pub input_muted: bool,
    pub output_muted: bool,
    pub frame_number: u64,
}

impl State {
    pub fn new(input_sender: Sender, output_sender: Sender) -> Self {
        let (sender, receiver) = mpsc::channel();
        let handle = std::thread::Builder::new()
            .name("audio_input_handler".to_string())
            .spawn(|| {
                audio_input_thread(input_sender, receiver);
            })
            .unwrap();
        let input = Handler { handle, sender };

        let (sender, receiver) = mpsc::channel();
        let handle = std::thread::Builder::new()
            .name("audio_output_handler".to_string())
            .spawn(|| {
                audio_output_thread(output_sender, receiver);
            })
            .unwrap();
        let output = Handler { handle, sender };

        Self {
            frame_number: 0,
            input,
            output,
            input_muted: false,
            output_muted: false,
        }
    }
}

struct Input {
    device: audio::Input<Stream>,
    encoder: libopus::Encoder,
    consumer: audio::Consumer,
    pcm_chunk: Vec<f32>,
    encoded_pcm: Vec<u8>,
}

pub type Sender = Box<dyn FnMut(Event) + Send>;

fn audio_input_thread(mut sender: Sender, receiver: mpsc::Receiver<Command>) {
    use cpal::traits::StreamTrait as _;
    use ringbuf::traits::Consumer as _;
    use ringbuf::traits::Observer as _;

    let timeout = std::time::Duration::from_millis(5);
    let mut input: Option<Input> = None;

    loop {
        let mut should_sleep = true;

        match receiver.try_recv() {
            Ok(cmd) => {
                should_sleep = false;
                match cmd {
                    Command::SetDevice(SetDevice { config }) => {
                        let sample_rate = config.sample_rate;
                        let channels = config.channels;
                        let frame_size = libopus::calc_frame_size(sample_rate, 10);

                        let encoder = libopus::Encoder::new(
                            sample_rate,
                            channels,
                            libopus::Application::Audio,
                        )
                        .unwrap();

                        let desired_len = (frame_size * channels as u32) as usize;

                        // reuse buffers if they exist
                        let (pcm_chunk, encoded_pcm) = input
                            .map(|i| {
                                let needs_new_pcm_buf = desired_len != i.pcm_chunk.len();
                                let pcm_chunk = if needs_new_pcm_buf {
                                    vec![0.0; desired_len]
                                } else {
                                    i.pcm_chunk
                                };
                                (pcm_chunk, i.encoded_pcm)
                            })
                            .unwrap_or_else(|| {
                                let pcm_chunk = vec![0.0; desired_len];
                                (pcm_chunk, vec![0u8; 4096])
                            });

                        let host = cpal::default_host();
                        let ring = ringbuf::HeapRb::<f32>::new(4096 * 2);

                        let (device, consumer) = audio::get_input(&host, &config, ring);

                        input = Some(Input {
                            encoded_pcm,
                            pcm_chunk,
                            encoder,
                            consumer,
                            device,
                        })
                    }
                    Command::Pause => {
                        if let Some(input) = input.as_mut() {
                            input.device.stream().pause().unwrap();
                        }
                    }
                    Command::Play => {
                        if let Some(input) = input.as_mut() {
                            input.device.stream().play().unwrap();
                        }
                    }
                    Command::PlayOpusAudio(_) => panic!("audio input can not play opus audio"),
                }
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => return,
        }

        if let Some(input) = input.as_mut() {
            let has_chunk = input.consumer.occupied_len() >= input.pcm_chunk.len();

            if has_chunk {
                should_sleep = false;
                let size = input.consumer.pop_slice(&mut input.pcm_chunk);
                if size != input.pcm_chunk.len() {
                    // TODO: did not have room to write
                    panic!("did not have enough data to fill buffer from input")
                }

                let size = input
                    .encoder
                    .encode_f32(&input.pcm_chunk, &mut input.encoded_pcm)
                    .unwrap();

                let data = input.encoded_pcm[..size].to_vec();
                (sender)(Event::Data(data))
            }
        }

        if should_sleep {
            std::thread::sleep(timeout);
        }
    }
}

struct Output {
    device: audio::Output<Stream>,
    decoder: libopus::Decoder,
    producer: audio::Producer,
    decoded_pcm: Vec<f32>,
}

fn audio_output_thread(mut sender: Sender, receiver: mpsc::Receiver<Command>) {
    use cpal::traits::StreamTrait as _;
    use ringbuf::traits::Producer as _;

    let mut output: Option<Output> = None;
    let timeout = std::time::Duration::from_millis(5);

    loop {
        match receiver.recv() {
            Ok(cmd) => match cmd {
                Command::SetDevice(SetDevice { config }) => {
                    let decoder =
                        libopus::Decoder::new(config.sample_rate, config.channels).unwrap();

                    let host = cpal::default_host();
                    let ring = ringbuf::HeapRb::<f32>::new(4096 * 2);

                    let (device, producer) = audio::get_output(&host, &config, ring);

                    // reuse buffer if it exists
                    let decoded_pcm = output
                        .map(|o| o.decoded_pcm)
                        .unwrap_or_else(|| vec![0.0; 4096]);

                    output = Some(Output {
                        decoder,
                        producer,
                        device,
                        decoded_pcm,
                    })
                }
                Command::Pause => {
                    if let Some(output) = output.as_mut() {
                        output.device.stream().pause().unwrap();
                    }
                }
                Command::Play => {
                    if let Some(output) = output.as_mut() {
                        output.device.stream().play().unwrap();
                    }
                }
                Command::PlayOpusAudio(data) => {
                    if let Some(output) = output.as_mut() {
                        let size = output
                            .decoder
                            .decode_f32(&data, &mut output.decoded_pcm, false)
                            .unwrap();

                        let pushed_count = output.producer.push_slice(&output.decoded_pcm[..size]);
                        if pushed_count != size {
                            // did not push all of the items
                        }
                    }
                }
            },
            Err(mpsc::RecvError) => return,
        }
    }
}
