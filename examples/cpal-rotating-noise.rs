//! A simple example to demonstrate the effect of HRTF on a real-time stream.
//!
//! The example generates a monophonic buffer of noise to be used as the sound source. A new source
//! position is created at the beginning of each call to the output stream's render function in
//! order to rotate the sound source around the user's head.
//!
//! The example will fail if the default cpal output device under the default host offers less than
//! two channels and cannot achieve a sample rate of 44.1 KHz.
//!
//! The effect is best experienced with headphones.

extern crate anyhow;
extern crate cpal;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rand::{rngs::SmallRng, SeedableRng};
use std::f32::consts::PI;

fn main() {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("failed to find a default output device");
    let mut config = device.default_output_config()?;

    // Humans have two ears.
    config.channels = 2;
    // The HRTFs are designed for 44.1 KHz
    config.sample_rate = cpal::SampleRate(44_100);

    match config.sample_format() {
        cpal::SampleFormat::F32 => run::<f32>(&device, &config.into()),
        cpal::SampleFormat::I16 => run::<i16>(&device, &config.into()),
        cpal::SampleFormat::U16 => run::<u16>(&device, &config.into()),
    }
}

// Run the stream with the specified format.
fn run<T>(device: &cpal::Device, config: &cpal::StreamConfig)
where
    T: cpal::Sample,
{
    let channels = config.channels as usize;
    let sample_rate = config.sample_rate.0 as f32;
    let volume = 0.25;
    let rotation_hz = 0.5;
    let mut stream_start = None;

    // The RNG used to generate the noise.
    let mut rng = rand::rngs::SmallRng::new();

    // Build the output stream.
    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);
    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], info: &cpal::OutputCallbackInfo| {
            // Use the timestamp to determine the new location of the source sound.
            let now = info.timestamp().playback;
            let start = *stream_start.get_or_insert(now);
            let since_start = now.duration_since(&start).unwrap();
            let secs = since_start.as_secs_f32();
            let radians = secs * rotation_hz * 2.0 * PI;
            let x = radians.cos();
            let z = radians.sin();

            // Create a monophonic buffer of noise. Normally we shouldn't dynamically allocate on
            // the audio thread like this, but it's just a quick demo.
            let frame_count = data.len() / channels;
            let noise: Vec<_> = (..frame_count).map(|_| rng.gen::<f32>()).collect();

            write_data(data, channels, &mut next_value)
        },
        err_fn,
    ).expect("failed to build output stream");
    stream.play().expect("failed to play stream");

    // Stop after 10 seconds.
    std::thread::sleep(std::time::Duration::from_secs(10));

    Ok(())
}

fn write_data<T>(output: &mut [T], channels: usize, rng: &mut dyn FnMut() -> f32)
where
    T: cpal::Sample,
{
    for frame in output.chunks_mut(channels) {
        let value: T = cpal::Sample::from::<f32>(&next_sample());
        for sample in frame.iter_mut() {
            *sample = value;
        }
    }
}
