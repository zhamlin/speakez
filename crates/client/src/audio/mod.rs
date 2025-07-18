mod input;
mod output;
pub mod state;

pub use input::Input;
pub use libopus::calc_frame_size;
pub use output::Output;
pub use state::State;

use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait as _};
use cpal::FromSample;
use cpal::Sample;
use ringbuf::storage::Heap;
use ringbuf::traits::Split as _;
use ringbuf::HeapRb;

pub type Producer = ringbuf::CachingProd<Arc<HeapRb<f32>>>;
pub type Consumer = ringbuf::CachingCons<Arc<HeapRb<f32>>>;
pub type Ringbuf = ringbuf::SharedRb<Heap<f32>>;

pub struct StreamEmpty;
pub struct Stream {
    stream: cpal::Stream,
}

#[derive(Debug)]
pub struct DeviceConfig {
    pub name: Option<String>,
    pub channels: u8,
    pub sample_rate: u32,
    pub buf_size: u32,
}

pub fn get_input(host: &cpal::Host, cfg: &DeviceConfig, buf: Ringbuf) -> (Input<Stream>, Consumer) {
    let device = match &cfg.name {
        None => host
            .default_input_device()
            .expect("failed to find input device"),
        Some(device) => host
            .input_devices()
            .unwrap()
            .find(|x| x.name().map(|y| y == *device).unwrap_or(false))
            .expect("failed to find input device"),
    };

    println!("Using input device: \"{}\"", device.name().unwrap());
    let config = device.default_input_config().unwrap();
    let buf_size = *config.buffer_size();

    let mut config: cpal::StreamConfig = config.into();
    config.sample_rate = cpal::SampleRate(cfg.sample_rate);
    config.channels = cfg.channels as u16;
    config.buffer_size = match buf_size {
        cpal::SupportedBufferSize::Range { min, max } => {
            if cfg.buf_size >= min && cfg.buf_size <= max {
                cpal::BufferSize::Fixed(cfg.buf_size)
            } else {
                config.buffer_size
            }
        }
        cpal::SupportedBufferSize::Unknown => config.buffer_size,
    };

    // Build streams.
    println!(
        "Attempting to build input stream with f32 samples and `{:?}`.",
        config
    );

    let (producer, consumer) = buf.split();
    let input = Input::new(device).build_stream(&config, producer);

    (input, consumer)
}

pub fn get_output(
    host: &cpal::Host,
    cfg: &DeviceConfig,
    buf: Ringbuf,
) -> (Output<Stream>, Producer) {
    let device = match &cfg.name {
        None => host
            .default_output_device()
            .expect("failed to find output device"),
        Some(device) => host
            .output_devices()
            .unwrap()
            .find(|x| x.name().map(|y| y == *device).unwrap_or(false))
            .expect("failed to find output device"),
    };

    println!("Using output device: \"{}\"", device.name().unwrap());

    let config = device.default_output_config().unwrap();
    let buf_size = *config.buffer_size();

    let mut config: cpal::StreamConfig = config.into();
    config.sample_rate = cpal::SampleRate(cfg.sample_rate);
    config.channels = cfg.channels as u16;
    config.buffer_size = match buf_size {
        cpal::SupportedBufferSize::Range { min, max } => {
            if cfg.buf_size >= min && cfg.buf_size <= max {
                cpal::BufferSize::Fixed(cfg.buf_size)
            } else {
                config.buffer_size
            }
        }
        cpal::SupportedBufferSize::Unknown => config.buffer_size,
    };

    // Build streams.
    println!(
        "Attempting to build outupt stream with f32 samples and `{:?}`.",
        config
    );

    let (producer, consmer) = buf.split();
    let output = Output::new(device).build_stream(&config, consmer);

    (output, producer)
}

pub fn list_devices() {
    let available_hosts = cpal::available_hosts();
    for host_id in available_hosts {
        println!("{}", host_id.name());
        let host = cpal::host_from_id(host_id).unwrap();

        let default_in = host.default_input_device().map(|e| e.name().unwrap());
        let default_out = host.default_output_device().map(|e| e.name().unwrap());
        println!("  Default Input Device:\n    {:?}", default_in);
        println!("  Default Output Device:\n    {:?}", default_out);

        let devices = host.devices().unwrap();
        println!("  Devices: ");
        for (device_index, device) in devices.enumerate() {
            println!("  {}. \"{}\"", device_index + 1, device.name().unwrap());
        }
    }
}

fn apply_gain<SampleType>(output: &mut [SampleType], value: f32, num_channels: usize)
where
    SampleType: Sample + FromSample<f32> + std::ops::MulAssign,
{
    let value: SampleType = SampleType::from_sample(value);
    for frame in output.chunks_mut(num_channels) {
        // copy the same value to all channels
        for sample in frame.iter_mut() {
            *sample *= value;
        }
    }
}
