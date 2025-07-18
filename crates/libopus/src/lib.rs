use opus_sys as opus;

pub fn calc_frame_size(sample_rate: u32, frame_duration_ms: u32) -> u32 {
    (sample_rate / 1000) * frame_duration_ms
}

pub enum ErrorCode {}

#[repr(u32)]
pub enum Application {
    VOIP = opus::OPUS_APPLICATION_VOIP,
    Audio = opus::OPUS_APPLICATION_AUDIO,
    RestrictedLowdelay = opus::OPUS_APPLICATION_RESTRICTED_LOWDELAY,
}

pub struct Encoder {
    inner: *mut opus::OpusEncoder,
    channels: u8,
}

unsafe impl Send for Encoder {}

impl Encoder {
    pub fn new(sample_rate: u32, channels: u8, application: Application) -> Result<Self, i32> {
        let mut error = 0;
        let encoder = unsafe {
            opus::opus_encoder_create(
                sample_rate as i32,
                channels as i32,
                application as i32,
                &mut error,
            )
        };
        if (error as u32) != opus::OPUS_OK || encoder.is_null() {
            Err(error)
        } else {
            Ok(Encoder {
                inner: encoder,
                channels,
            })
        }
    }

    pub fn encode_f32(&mut self, input: &[f32], output: &mut [u8]) -> Result<usize, i32> {
        let ret = unsafe {
            opus::opus_encode_float(
                self.inner,
                input.as_ptr(),
                input.len() as i32 / self.channels as i32,
                output.as_mut_ptr(),
                output.len() as i32,
            )
        };
        if ret < 0 {
            Err(ret)
        } else {
            Ok(ret as usize)
        }
    }
}

impl Drop for Encoder {
    fn drop(&mut self) {
        unsafe {
            opus::opus_encoder_destroy(self.inner);
        }
    }
}

unsafe impl Send for Decoder {}

pub struct Decoder {
    inner: *mut opus::OpusDecoder,
    channels: u8,
}

impl Decoder {
    pub fn new(sample_rate: u32, channels: u8) -> Result<Self, i32> {
        let mut error = 0;
        let decoder =
            unsafe { opus::opus_decoder_create(sample_rate as i32, channels as i32, &mut error) };
        if (error as u32) != opus::OPUS_OK || decoder.is_null() {
            Err(error)
        } else {
            Ok(Decoder {
                inner: decoder,
                channels,
            })
        }
    }

    pub fn decode_f32(
        &mut self,
        input: &[u8],
        output: &mut [f32],
        fec: bool,
    ) -> Result<usize, i32> {
        let ret = unsafe {
            opus::opus_decode_float(
                self.inner,
                input.as_ptr(),
                input.len() as i32,
                output.as_mut_ptr(),
                output.len() as i32,
                fec as i32,
            )
        };
        if ret < 0 {
            Err(ret)
        } else {
            Ok((ret * self.channels as i32) as usize)
        }
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        unsafe {
            opus::opus_decoder_destroy(self.inner);
        }
    }
}
