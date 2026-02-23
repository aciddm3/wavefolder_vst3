use nih_plug::prelude::*;

use nih_plug_egui::EguiState;
use parking_lot::RwLock;
use std::sync::Arc;

pub const MAX_GAIN: f32 = 120.0;
pub const MIN_GAIN: f32 = -60.0;

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
    #[id = "interpolation_method"]
    pub interpolation_method: IntParam,
    #[id = "bias"]
    pub bias: FloatParam,
    #[id = "func_gain"]
    pub func_gain: FloatParam,
    #[id = "output_clipping_enable"]
    pub clipping_enable: BoolParam,
    #[persist = "waveform_path"]
    pub waveform_path: RwLock<String>,
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,
}

impl Default for WFParams {
    fn default() -> Self {
        Self {
            gain: FloatParam::new(
                "Drive",
                0.0,
                FloatRange::Linear {
                    min: MIN_GAIN,
                    max: MAX_GAIN,
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
            interpolation_method: IntParam::new(
                "Interpolation_method",
                0,
                IntRange::Linear { min: 0, max: 1 },
            )
            .with_value_to_string(Arc::new(|s| {
                match s {
                    0 => "Linear",
                    1 => "Cosine",
                    _ => "Err",
                }
                .to_string()
            })),
            waveform_path: RwLock::new(String::new()),
            editor_state: EguiState::from_size(740, 550),
            bias: FloatParam::new(
                "Bias",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            ),
            func_gain: FloatParam::new(
                "Dry post gain",
                0.0,
                FloatRange::Linear {
                    min: MIN_GAIN,
                    max: MAX_GAIN,
                },
            ),
            clipping_enable: BoolParam::new("Clipping enable", true),
        }
    }
}
