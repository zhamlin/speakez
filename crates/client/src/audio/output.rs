use ringbuf::traits::Consumer as _;

use cpal::traits::DeviceTrait;

use super::{Consumer, Stream, StreamEmpty};

fn err_fn(err: cpal::StreamError) {
    eprintln!("an error occurred on stream: {}", err);
}

pub struct Output<S> {
    device: cpal::Device,
    state: S,
}

impl Output<StreamEmpty> {
    pub fn new(device: cpal::Device) -> Self {
        Self {
            device,
            state: StreamEmpty,
        }
    }

    pub fn build_stream(
        self,
        config: &cpal::StreamConfig,
        mut consumer: Consumer,
    ) -> Output<Stream> {
        let output_data_fn = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            let mut input_fell_behind = false;
            for sample in data {
                *sample = match consumer.try_pop() {
                    Some(s) => s,
                    None => {
                        input_fell_behind = true;
                        0.0
                    }
                };
            }
            if input_fell_behind {
                // eprintln!("input stream fell behind: try increasing latency");
            }
        };

        let stream = self
            .device
            .build_output_stream(config, output_data_fn, err_fn, None)
            .unwrap();

        Output {
            device: self.device,
            state: Stream { stream },
        }
    }
}

impl Output<Stream> {
    pub fn stream(&self) -> &cpal::Stream {
        &self.state.stream
    }

    pub fn destroy_stream(self) -> Output<StreamEmpty> {
        Output {
            device: self.device,
            state: StreamEmpty,
        }
    }
}
