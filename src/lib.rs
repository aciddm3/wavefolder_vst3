use egui::RichText;
use nih_plug::plugin::vst3::Vst3Plugin; // Импортируем Vst3Plugin
use nih_plug::prelude::*; // Импортируем все необходимые трейты и типы из nih-plug [16]
use nih_plug::wrapper::vst3::subcategories::Vst3SubCategory; // Импортируем Vst3SubCategory из правильного пути
use nih_plug_egui::{EguiState, create_egui_editor, egui, widgets};
use parking_lot::RwLock;
use std::sync::Arc;

mod utils;
mod wav_reader;
mod wf_params;
mod zero_crossing_detector;

struct WF {
    params: Arc<wf_params::WFParams>,
    last_open_file_state: bool,
    // Используем RwLock вместо ArcSwap для стабильности в Ardour
    custom_waveform: Arc<RwLock<Arc<Vec<f32>>>>,
    editor_state: Arc<EguiState>,
    zero_crossing_points: Arc<RwLock<Vec<f32>>>,
    zc_input_buffer: Arc<RwLock<String>>,
    trunked_val: Arc<RwLock<usize>>,
}

impl Default for WF {
    fn default() -> Self {
        // Создаем начальную таблицу сразу в Default
        let default_table = (0..2048)
            .map(|s| s as f32 / 1024.0 - 1.0)
            .collect::<Vec<_>>();
        Self {
            params: Arc::new(wf_params::WFParams::default()),
            last_open_file_state: false,
            custom_waveform: Arc::new(RwLock::new(Arc::new(default_table))),
            editor_state: EguiState::from_size(740, 475),
            zero_crossing_points: Arc::new(RwLock::new(vec![0.5])),
            zc_input_buffer: Arc::new(RwLock::new(String::new())),
            trunked_val: Arc::new(RwLock::new(0)),
        }
    }
}

enum WFBackgroundTask {
    LoadFile,
    LoadFileNoDialog,
}

impl Plugin for WF {
    type SysExMessage = ();
    type BackgroundTask = WFBackgroundTask;

    const NAME: &'static str = "WaveFolder distortion";
    const VENDOR: &'static str = "Gemma";
    const URL: &'static str = "https://example.com/wavefolder-distortion";
    const EMAIL: &'static str = "None";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(1),
        main_output_channels: NonZeroU32::new(1),
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
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        self.zc_input_buffer = Arc::new(RwLock::new(String::new()));
        self.trunked_val = Arc::new(RwLock::new(0));
        self.last_open_file_state = false;

        let default_table = (0..2048)
            .map(|s| s as f32 / 1024.0 - 1.0)
            .collect::<Vec<_>>();
        *self.custom_waveform.write() = Arc::new(default_table);
        *self.zero_crossing_points.write() = vec![0.5];
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
        let table_lock = self.custom_waveform.read();
        let custom_table = &**table_lock; // &[f32]

        for channel_samples in buffer.as_slice() {
            for sample in channel_samples.iter_mut() {
                let gain = utils::db_to_gain(self.params.gain.smoothed.next());
                let phase_offset = self.params.phase.smoothed.next() / 90.0;
                let dry_wet = self.params.dw.smoothed.next();

                let input_folded = *sample * gain + phase_offset;

                let wet = match self.params.waveform.value() {
                    0 => utils::sine(input_folded),
                    1 => utils::triangle(input_folded),
                    2 => utils::saw(input_folded),
                    3 => utils::meander(input_folded),
                    4 => utils::lookup_custom(custom_table, input_folded),
                    _ => utils::sine(input_folded),
                };

                *sample = utils::xfader(*sample, wet, dry_wet);
            }
        }

        ProcessStatus::Normal
    }

    fn task_executor(&mut self) -> TaskExecutor<Self> {
        let params = self.params.clone();
        let custom_waveform = self.custom_waveform.clone();
        let zero_crossing_points = self.zero_crossing_points.clone();
        Box::new(move |task| {
            match task {
                WFBackgroundTask::LoadFileNoDialog => {
                    let path_str = params.waveform_path.read().clone();
                    if !path_str.is_empty() {
                        // Здесь вызываем загрузку (внутри будет lock.write())
                        wav_reader::process_wav_from_path(
                            &path_str,
                            &custom_waveform,
                            &zero_crossing_points,
                        );
                    }
                }
                WFBackgroundTask::LoadFile => {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("WAV", &["wav"])
                        .pick_file()
                    {
                        let path_str = path.to_string_lossy().into_owned();
                        wav_reader::process_wav_from_path(
                            &path_str,
                            &custom_waveform,
                            &zero_crossing_points,
                        );
                        *params.waveform_path.write() = path_str;
                    }
                }
            }
        })
    }

    fn editor(&mut self, async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let params = self.params.clone();
        let waveform_arc = self.custom_waveform.clone();
        let zc_points_arc = self.zero_crossing_points.clone();
        let string_buffer_arc = self.zc_input_buffer.clone();
        let trunked_value_arc = self.trunked_val.clone();

        const ZERO_CROSSING_LINE_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 0, 255);
        const PHASE_LINE_COLOR: egui::Color32 = egui::Color32::from_rgb(128, 128, 255);
        const ZERO_CROSSING_LINE_TEXT_COLOR: egui::Color32 = egui::Color32::from_rgb(127, 255, 127);
        const PHASE_LINE_TEXT_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 255, 128);
        const GRAPH_LINE_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 0, 0);
        const POSITIVE_COLOR: egui::Color32 = egui::Color32::from_rgb(0, 240, 0);
        const NEGATIVE_COLOR: egui::Color32 = egui::Color32::from_rgb(240, 0, 0);

        create_egui_editor(
            self.editor_state.clone(),
            (),
            |_ctx, _data| {},
            move |egui_ctx, setter, _data| {
                egui::CentralPanel::default().show(egui_ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading(
                            egui::RichText::new("WAVEFOLDER DISTORTION")
                                .strong()
                                .size(20.0),
                        );
                    });

                    ui.add_space(15.0);

                    // --- 1. ВИЗУАЛИЗАЦИЯ ГРАФИКА ---
                    let available_width = ui.available_width();
                    let (rect, _response) = ui.allocate_at_least(
                        egui::vec2(available_width, 120.0),
                        egui::Sense::hover(),
                    );
                    let painter = ui.painter_at(rect);

                    // Фон графика
                    painter.rect_filled(rect, 4.0, egui::Color32::from_black_alpha(30));

                    // Читаем данные из RwLock

                    let table_guard = waveform_arc.read(); // Блокируем один раз
                    let samples = &**table_guard; // Разыменовываем до &[f32]
                    let zc_points = zc_points_arc.read();
                    let mut trunked_val = trunked_value_arc.write();
                    if !samples.is_empty() {
                        let mid_y = rect.center().y;
                        let height_scale = rect.height() * 0.4;
                        let width = rect.width();

                        // --- 1. БЕЛЫЕ ОСИ (каждые 30°) ---
                        let grid_stroke =
                            egui::Stroke::new(0.5, egui::Color32::from_white_alpha(50));
                        for deg in (0..=360).step_by(30) {
                            let x_norm = deg as f32 / 360.0;
                            let x_pos = rect.left() + x_norm * width;
                            painter.line_segment(
                                [
                                    egui::pos2(x_pos, rect.top()),
                                    egui::pos2(x_pos, rect.bottom()),
                                ],
                                grid_stroke,
                            );
                        }
                        // Горизонтальная ось Y=0
                        painter.line_segment(
                            [
                                egui::pos2(rect.left(), mid_y),
                                egui::pos2(rect.right(), mid_y),
                            ],
                            grid_stroke,
                        );

                        // --- 2. ПОИСК НУЛЕЙ И ОТРИСОВКА ПУРПУРНЫХ ОСЕЙ ---

                        let mut points = Vec::with_capacity(width as usize);

                        for i in 0..width as usize {
                            let t = (i as f32 / width) * 4.0;
                            let sample = utils::lookup_custom(samples, t);

                            let x = rect.left() + i as f32;
                            let y = mid_y - (sample * height_scale);
                            points.push(egui::pos2(x, y));
                        }

                        for &val in zc_points.iter() {
                            {
                                let x = rect.left() + val * rect.width();
                                // Рисуем вертикальную линию
                                painter.line_segment(
                                    [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                                    egui::Stroke::new(
                                        1.0,
                                        ZERO_CROSSING_LINE_COLOR.linear_multiply(0.5),
                                    ),
                                );

                                // Подписываем градусы
                                let degrees = (val * 360.0).round() as i32;
                                painter.text(
                                    egui::pos2(x, rect.bottom() - 5.0),
                                    egui::Align2::CENTER_BOTTOM,
                                    format!("{}°", degrees),
                                    egui::FontId::monospace(9.0),
                                    ZERO_CROSSING_LINE_TEXT_COLOR,
                                );
                            }
                        }

                        {
                            let val = params.phase.value() / 360.0;
                            let x = rect.left() + val * rect.width();
                            // Рисуем вертикальную линию
                            painter.line_segment(
                                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                                egui::Stroke::new(
                                    1.0,
                                    PHASE_LINE_COLOR.linear_multiply(0.5),
                                ),
                            );

                            // Подписываем градусы
                            let degrees = (val * 360.0).round() as i32;
                            painter.text(
                                egui::pos2(x, rect.bottom() - 5.0),
                                egui::Align2::CENTER_BOTTOM,
                                format!("{}°", degrees),
                                egui::FontId::monospace(9.0),
                                PHASE_LINE_TEXT_COLOR,
                            );
                        }

                        // --- 3. САМА ЛИНИЯ ГРАФИКА ---
                        painter.add(egui::Shape::line(
                            points,
                            egui::Stroke::new(2.0, GRAPH_LINE_COLOR),
                        ));
                    }

                    ui.add_space(10.0);

                    // --- ВЫБОР ТОЧКИ ФАЗЫ ---
                    if !zc_points.is_empty() {
                        ui.horizontal(|ui| {
                            ui.label("Snap Phase to:");
                            let mut string_buffer = string_buffer_arc.write();
                            ui.text_edit_singleline(&mut *string_buffer);
                            if let Ok(val) = string_buffer.parse::<isize>() {
                                ui.label(
                                    RichText::new("A correct value was entered")
                                        .color(POSITIVE_COLOR),
                                );
                                *trunked_val = if val < 0 {
                                    0
                                } else if val > zc_points.len() as isize - 1 {
                                    zc_points.len()
                                } else {
                                    val as usize
                                };
                            } else {
                                ui.label(
                                    RichText::new("A incorrect value was entered")
                                        .color(NEGATIVE_COLOR),
                                );
                            }

                            if ui.button("SNAP!").clicked() {
                                let val = zc_points.get(*trunked_val).unwrap_or(&0.0) * 360.0;
                                setter.begin_set_parameter(&params.phase);
                                setter.set_parameter(&params.phase, val);
                                setter.end_set_parameter(&params.phase);
                            }
                        });
                    }

                    ui.add_space(15.0);
                    ui.horizontal(|ui| {
                        // --- 2. РАДИОКНОПКИ ВЫБОРА ВОЛНЫ ---
                        ui.group(|ui| {
                            ui.label("Waveform Type:");
                            ui.horizontal_wrapped(|ui| {
                                let mut current_wave = params.waveform.value();

                                // Создаем радиокнопки для каждого типа
                                for (val, label) in [
                                    (0, "Sine"),
                                    (1, "Triangle"),
                                    (2, "Saw"),
                                    (3, "Square"),
                                    (4, "From WAV"),
                                ] {
                                    if ui.radio_value(&mut current_wave, val, label).changed() {
                                        setter.begin_set_parameter(&params.waveform);
                                        setter.set_parameter(&params.waveform, current_wave);
                                        setter.end_set_parameter(&params.waveform);
                                    }
                                }
                            });
                        });

                        // --- 3. КНОПКА ВЫБОРА ФАЙЛА ---
                        ui.horizontal(|ui| {
                            if ui.button("Load WAV").clicked() {
                                // Используем переданный экзекутор для вызова диалога
                                async_executor.execute_background(WFBackgroundTask::LoadFile);
                            }
                        });

                        // Вывод текущего пути (если есть)
                        let path = params.waveform_path.read();
                        if !path.is_empty() {
                            let filename = std::path::Path::new(&*path)
                                .file_name()
                                .and_then(|f| f.to_str())
                                .unwrap_or("Unknown");
                            ui.label(egui::RichText::new(filename).italics().size(10.0));
                        }
                    });

                    ui.add_space(10.0);
                    // Слайдеры
                    let slider_size = egui::vec2(ui.available_width(), 20.0);

                    ui.label(egui::RichText::new("Dry/Wet"));
                    ui.add_sized(
                        slider_size,
                        widgets::ParamSlider::for_param(&params.dw, setter),
                    );
                    ui.add_space(5.0);
                    ui.label(egui::RichText::new("Drive"));
                    ui.add_sized(
                        slider_size,
                        widgets::ParamSlider::for_param(&params.gain, setter),
                    );
                    ui.add_space(5.0);
                    ui.label(egui::RichText::new("Phase"));
                    ui.add_sized(
                        slider_size,
                        widgets::ParamSlider::for_param(&params.phase, setter),
                    );
                });
            },
        )
    }
}

impl Vst3Plugin for WF {
    const VST3_CLASS_ID: [u8; 16] = [
        98, 218, 94, 45, 78, 44, 74, 204, 167, 126, 143, 79, 37, 188, 235, 20,
    ]; // UUID is generated randomly
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Distortion];
}

nih_export_vst3!(WF);
