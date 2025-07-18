use libopus::*;

#[test]
fn test_encoder_encode_f32() {
    let sample_rate = 48000;
    let frame_duration = 20;
    let frame_size = calc_frame_size(sample_rate, frame_duration);

    let mut e = Encoder::new(sample_rate, 2, Application::Audio).unwrap();
    let pcm = vec![0.0; (frame_size * 2) as usize];
    let mut output = vec![0; 256];
    let size = e.encode_f32(&pcm, &mut output).unwrap();
    assert_eq!(&output[..size], [252, 255, 254]);

    let mut e = Encoder::new(sample_rate, 1, Application::Audio).unwrap();
    let pcm = vec![0.0; frame_size as usize];
    let mut output = vec![0; 256];
    let size = e.encode_f32(&pcm, &mut output).unwrap();
    assert_eq!(&output[..size], [248, 255, 254]);
}

#[test]
fn test_encoder_and_decoder() {
    let sample_rate = 48000;
    let frame_duration = 20;
    let frame_size = calc_frame_size(sample_rate, frame_duration);

    let mut e = Encoder::new(sample_rate, 2, Application::Audio).unwrap();

    let pcm = vec![0.0; frame_size as usize];
    let mut output = vec![0; 256];
    let size = e.encode_f32(&pcm, &mut output).unwrap();
    assert_eq!(&output[..size], [244, 255, 254]);

    let mut decoder_output = vec![0.0; 2048];
    let mut decoder = Decoder::new(sample_rate, 2).unwrap();
    decoder
        .decode_f32(&output[..size], &mut decoder_output[..], false)
        .unwrap();
}
