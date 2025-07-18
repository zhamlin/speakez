use ringbuf::traits::Producer as _;

use cpal::traits::DeviceTrait;

use super::{Producer, Stream, StreamEmpty};

fn err_fn(err: cpal::StreamError) {
    eprintln!("an error occurred on stream: {}", err);
}

pub struct Input<S> {
    device: cpal::Device,
    state: S,
}

impl Input<StreamEmpty> {
    pub fn new(device: cpal::Device) -> Self {
        Self {
            device,
            state: StreamEmpty,
        }
    }

    pub fn build_stream(
        self,
        config: &cpal::StreamConfig,
        mut producer: Producer,
    ) -> Input<Stream> {
        let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let mut output_fell_behind = false;
            for &sample in data {
                if producer.try_push(sample).is_err() {
                    output_fell_behind = true;
                }
            }
            if output_fell_behind {
                eprintln!("output stream fell behind: try increasing latency");
            }
        };

        let stream = self
            .device
            .build_input_stream(config, input_data_fn, err_fn, None)
            .unwrap();

        Input {
            device: self.device,
            state: Stream { stream },
        }
    }
}

impl Input<Stream> {
    pub fn stream(&self) -> &cpal::Stream {
        &self.state.stream
    }

    pub fn destroy_stream(self) -> Input<StreamEmpty> {
        Input {
            device: self.device,
            state: StreamEmpty,
        }
    }
}
