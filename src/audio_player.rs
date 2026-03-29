use cpal::traits::{DeviceTrait as _, HostTrait as _, StreamTrait as _};

pub struct AudioPlayer {
    host: cpal::platform::Host,
    device: cpal::platform::Device,
    output_streams: Vec<cpal::platform::Stream>,
    audio_files: std::sync::Arc<std::sync::Mutex<Vec<Vec<f32>>>>,
}

impl AudioPlayer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let host = cpal::default_host();

        const BUFFER_SIZE_PER_CHANNEL: u32 = 4096 * 4;

        let device = host
            .default_output_device()
            .ok_or_else(|| "No audio device available.".to_owned())?;

        let mut supported_configs_range = device
            .supported_output_configs()
            .expect("error while querying configs");
        let supported_config = supported_configs_range
            .next()
            .expect("no supported config?!")
            .with_max_sample_rate();

        let mut player = Self {
            host,
            device,
            output_streams: vec![],
            audio_files: Default::default(),
        };

        let audio_files_clone = player.audio_files.clone();

        let stream = player
            .device
            .build_output_stream(
                &cpal::StreamConfig {
                    channels: 2,
                    sample_rate: 44100,
                    buffer_size: cpal::BufferSize::Fixed(BUFFER_SIZE_PER_CHANNEL),
                },
                // &supported_config.config(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    audio_files_clone
                        .lock()
                        .expect("Bad mutex")
                        .retain_mut(|slice| {
                            // Write the slice into the buffer
                            slice
                                .iter()
                                .take(data.len().min(2 * BUFFER_SIZE_PER_CHANNEL as usize))
                                .enumerate()
                                .for_each(|(i, sample)| data[i] += sample);

                            *slice = slice.split_off(2 * BUFFER_SIZE_PER_CHANNEL as usize);

                            // Keep unfinished slices
                            !slice.is_empty()
                        });
                },
                move |err| {
                    eprintln!("{err}");
                },
                None, // None=blocking, Some(Duration)=timeout
            )
            .expect("Failed to create stream");

        stream.play().expect("failed to play stream");
        player.output_streams.push(stream);

        Ok(player)
    }

    pub fn restart(&self) {
        for output_stream in &self.output_streams {
            output_stream.play().unwrap();
        }
    }

    pub fn add_audio(&self, audio: &[f32]) {
        self.audio_files
            .lock()
            .expect("Failed to lock audio")
            .push(audio.to_vec());

        self.output_streams
            .iter()
            .for_each(|stream| stream.play().unwrap());
    }
}
