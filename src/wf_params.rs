use nih_plug::prelude::*;

use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Params)]
pub struct WFParams {
    #[id = "gain"]
    pub gain: FloatParam,
    #[id = "phase"]
    pub phase: FloatParam,
    #[id = "drywet"]
    pub dw: FloatParam,
    #[id = "waveform"]
    pub waveform: IntParam,
    #[persist = "waveform_path"]
    pub waveform_path: RwLock<String>,
}

impl Default for WFParams {
    fn default() -> Self {
        Self {
            gain: FloatParam::new(
                "Drive",
                0.0,
                FloatRange::Linear {
                    min: -60.0,
                    max: 120.0,
                },
            )
            .with_unit("dB"),
            phase: FloatParam::new(
                "Phase",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 360.0,
                },
            )
            .with_unit("deg"),
            dw: FloatParam::new("Dry/Wet", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            waveform: IntParam::new("Waveform", 1, IntRange::Linear { min: 0, max: 4 })
                .with_value_to_string(Arc::new(|s| {
                    match s {
                        0 => "Sine",
                        1 => "Triangle",
                        2 => "Saw",
                        3 => "Square",
                        4 => "Custom (file)",
                        _ => "How has you entered this value? (>O_o<)",
                    }
                    .to_string()
                })),
            waveform_path: RwLock::new(String::new()),
        }
    }
}
