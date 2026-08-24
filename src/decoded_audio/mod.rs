pub mod load_file;
pub mod process_file;

#[derive(Debug, Clone)]
pub struct DecodedAudio {
    pub samples: Vec<Vec<f32>>,
    current_channel: usize,
}

impl DecodedAudio {
    pub fn new(samples: Vec<Vec<f32>>) -> Self {
        let current_channel = 0;
        Self {
            samples,
            current_channel,
        }
    }

    pub fn get_channel_count(&self) -> usize {
        self.samples.len()
    }

    pub fn get_current_channel(&self) -> usize {
        self.current_channel
    }

    pub fn set_audio_channel(&mut self, channel_number: usize) {
        self.current_channel = channel_number.clamp(0, self.samples.len() - 1);
    }
}

impl Default for DecodedAudio {
    fn default() -> Self {
        Self {
            samples: vec![vec![0.0, 1.0, -1.0, 0.0]],
            current_channel: 0,
        }
    }
}
