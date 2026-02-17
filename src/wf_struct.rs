use crate::wf_params;
use std::sync::Arc;
use parking_lot::RwLock;

pub struct WF {
    pub params: Arc<wf_params::WFParams>,
    pub last_open_file_state: bool,
    pub custom_waveform: Arc<RwLock<Arc<Vec<f32>>>>,
}

impl Default for WF {
    fn default() -> Self {
        let default_table = (0..2048)
            .map(|s| s as f32 / 1024.0 - 1.0)
            .collect::<Vec<_>>();
        Self {
            params: Arc::new(wf_params::WFParams::default()),
            last_open_file_state: false,
            custom_waveform: Arc::new(RwLock::new(Arc::new(default_table))),
        }
    }
}