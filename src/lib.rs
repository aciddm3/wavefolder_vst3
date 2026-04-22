use nih_plug::plugin::vst3::Vst3Plugin; // Импортируем Vst3Plugin
use nih_plug::prelude::*; // Импортируем все необходимые трейты и типы из nih-plug [16]
use nih_plug::wrapper::vst3::subcategories::Vst3SubCategory; // Импортируем Vst3SubCategory из правильного пути
use std::sync::Arc;

mod biquad;
mod file_reader;
mod func;
mod gui;
mod interpolation;
mod utils;
mod wf_background_task;
mod wf_params;
mod wf_struct;

use crate::wf_background_task::WFBackgroundTask;
use crate::wf_struct::WF;

impl Plugin for WF {
    type SysExMessage = ();
    type BackgroundTask = WFBackgroundTask;

    const NAME: &'static str = "WaveFolder distortion";
    const VENDOR: &'static str = "Gemma";
    const URL: &'static str = "https://example.com/wavefolder-distortion";
    const EMAIL: &'static str = "None";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        aux_input_ports: &[],
        aux_output_ports: &[],
        names: PortNames::const_default(),
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        let a = self.params.clone();

        self.last_open_file_state = false;

        let num_channels = if let Some(val) = audio_io_layout.main_input_channels {
            val.get()
        } else {
            2
        } as usize;

        *self = Self::new(buffer_config.sample_rate, 4, num_channels);

        let default_table = (-1..=1).map(|s| s as f32).collect::<Vec<_>>();

        self.params = a;

        *self.custom_waveform.write() = Arc::new(default_table);
        let path = self.params.waveform_path.read().clone();
        if !path.is_empty() {
            context.execute(WFBackgroundTask::LoadFileNoDialog);
        }

        true
    }

    fn reset(&mut self) {}

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let num_samples = buffer.samples();
        let table_lock = self.custom_waveform.read();
        let custom_table = &**table_lock; // &[f32]

        for sample_index in 0..num_samples {
            let dry_wet = self.params.dw.smoothed.next();
            let waveform_type = self.params.waveform.value();
            let interpolation_method = self.params.interpolation_method.value();
            let drive = self.params.gain.smoothed.next();
            let phase = self.params.phase.smoothed.next();
            let func_gain = self.params.func_gain.smoothed.next();
            let bias = self.params.bias.smoothed.next();
            let clipping_enable = self.params.clipping_enable.value();

            for (channel_idx, channel) in buffer.as_slice().iter_mut().enumerate() {
                self.resamplers[channel_idx].push_sample(channel[sample_index]);
                for jndex in 0..self.oversampling {
                    let mut wet = self.resamplers[channel_idx].get_phase(jndex);

                    // processing
                    wet = func::func(
                        wet,
                        waveform_type,
                        interpolation_method,
                        drive,
                        phase,
                        func_gain,
                        bias,
                        custom_table,
                    );

                    if clipping_enable {
                        wet = wet.clamp(-1.0, 1.0)
                    }
                    // processing end

                    wet = self.anti_al_filter1[channel_idx].process(wet);
                    wet = self.anti_al_filter2[channel_idx].process(wet);
                    self.decimators[channel_idx].process_oversampled(wet);
                }
                channel[sample_index] = utils::xfader(
                    channel[sample_index],
                    self.decimators[channel_idx].get_output(),
                    dry_wet,
                );
            }
        }

        ProcessStatus::Normal
    }

    fn task_executor(&mut self) -> TaskExecutor<Self> {
        let params = self.params.clone();
        let custom_waveform = self.custom_waveform.clone();
        Box::new(move |task| match task {
            WFBackgroundTask::LoadFileNoDialog => {
                let path_str = params.waveform_path.read().clone();
                if !path_str.is_empty() {
                    if let Err(e) = file_reader::process_file_from_path(&path_str, &custom_waveform)
                    {
                        nih_log!("{e}");
                    }
                }
            }
            WFBackgroundTask::LoadFile => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("WAV, FLAC, OGG", &["wav", "flac", "ogg"])
                    .pick_file()
                {
                    let path_str = path.to_string_lossy().into_owned();

                    if let Err(e) = file_reader::process_file_from_path(&path_str, &custom_waveform)
                    {
                        nih_log!("{e}");
                    }
                    *params.waveform_path.write() = path_str;
                }
            }
        })
    }

    fn editor(&mut self, async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        self.wf_gui(async_executor)
    }
}

impl Vst3Plugin for WF {
    const VST3_CLASS_ID: [u8; 16] = [
        98, 218, 94, 45, 78, 44, 74, 204, 167, 126, 143, 79, 37, 188, 235, 20,
    ]; // UUID is generated randomly
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Distortion, Vst3SubCategory::Mono];
}

nih_export_vst3!(WF);
