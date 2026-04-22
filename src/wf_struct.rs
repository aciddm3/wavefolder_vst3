use crate::{biquad, interpolation, wf_params};
use parking_lot::RwLock;
use std::sync::Arc;

pub struct WF {
    pub params: Arc<wf_params::WFParams>,
    pub last_open_file_state: bool,

    pub anti_al_filter1: Vec<biquad::Biquad>,
    pub anti_al_filter2: Vec<biquad::Biquad>,
    
    pub resamplers: Vec<interpolation::SincResampler>,
    pub oversampling: usize,
    pub decimators: Vec<interpolation::SincDecimator>,
    
    pub custom_waveform: Arc<RwLock<Arc<Vec<f32>>>>,
}

impl WF {
    pub fn new(sample_rate: f32, oversampling: usize, num_channels: usize) -> Self {
        let mut anti_al_filter1 = biquad::Biquad::new();
        let mut anti_al_filter2 = biquad::Biquad::new();
        anti_al_filter1.set_lowpass(sample_rate / 2.0, 0.5412, sample_rate * oversampling as f32);
        anti_al_filter2.set_lowpass(sample_rate / 2.0, 1.3065, sample_rate * oversampling as f32);

        let default_table = (0..=2).map(|s| s as f32 - 1.0).collect::<Vec<_>>();
        Self {
            params: Arc::new(wf_params::WFParams::default()),
            last_open_file_state: false,
            custom_waveform: Arc::new(RwLock::new(Arc::new(default_table))),
            anti_al_filter1: vec![anti_al_filter1; num_channels],
            anti_al_filter2: vec![anti_al_filter2; num_channels],
            resamplers: vec![interpolation::SincResampler::new(oversampling, 8); num_channels],
            decimators: vec![interpolation::SincDecimator::new(oversampling, 8); num_channels],
            oversampling,
        }
    }
}

impl Default for WF {
    fn default() -> Self {
        Self::new(44100.0, 2, 2)
    }
}
