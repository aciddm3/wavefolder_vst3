use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui, widgets};

//const ZERO_CROSSING_LINE_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 0, 255);
//const PHASE_LINE_TEXT_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 255, 128);
const ZERO_CROSSING_LINE_TEXT_COLOR: egui::Color32 = egui::Color32::from_rgb(127, 255, 127);
const PHASE_LINE_COLOR: egui::Color32 = egui::Color32::from_rgb(128, 128, 255);
const GRAPH_LINE_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 0, 0);

use crate::wf_background_task::WFBackgroundTask;
use crate::utils;
use crate::wf_struct::WF;

impl WF {
    pub fn wf_gui(&mut self, async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let params = self.params.clone();
        let waveform_arc = self.custom_waveform.clone();

        create_egui_editor(
            self.params.editor_state.clone(),
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

                    let available_width = ui.available_width();

                    let (rect, response) = ui
                        .allocate_at_least(egui::vec2(available_width, 120.0), egui::Sense::drag());

                    if response.dragged() {
                        let delta_x = response.drag_delta().x;

                        let phase_delta = -(delta_x / rect.width())
                            * params.gain.value().log(2.0).max(4.0)
                            * 12.0;

                        let mut new_phase = params.phase.value() - phase_delta;

                        new_phase = new_phase.rem_euclid(360.0);

                        // Устанавливаем новое значение через setter
                        setter.begin_set_parameter(&params.phase);
                        setter.set_parameter(&params.phase, new_phase);
                        setter.end_set_parameter(&params.phase);
                    }

                    if response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                        let scroll_delta =
                            -ui.input(|i| i.smooth_scroll_delta.y + i.raw_scroll_delta.y);
                        let sensitivity = if ui.input(|i| i.modifiers.command || i.modifiers.ctrl) {
                            0.001
                        } else {
                            0.1
                        };
                        use crate::wf_params::{MAX_GAIN, MIN_GAIN};
                        let new_gain = (params.gain.value() + scroll_delta * sensitivity)
                            .clamp(MIN_GAIN, MAX_GAIN);
                        setter.begin_set_parameter(&params.gain);
                        setter.set_parameter(&params.gain, new_gain);
                        setter.end_set_parameter(&params.gain);
                    }

                    let painter = ui.painter_at(rect);

                    // Фон графика
                    painter.rect_filled(rect, 4.0, egui::Color32::from_black_alpha(30));

                    // Читаем данные из RwLock

                    let table_guard = waveform_arc.read(); // Блокируем один раз
                    let samples = &**table_guard; // Разыменовываем до &[f32]

                    if !samples.is_empty() {
                        let mid_y = rect.center().y;
                        let height_scale = rect.height() * 0.4;
                        let width = rect.width();

                        // --- 1. БЕЛЫЕ ОСИ (каждые 30°) ---
                        let grid_stroke =
                            egui::Stroke::new(0.5, egui::Color32::from_white_alpha(50));
                        for x in [-1.25, -1.0, -0.75, -0.5, -0.25, 0.25, 0.5, 0.75, 1.0, 1.25] {
                            let x_norm = x / 2.5 + 0.5;
                            let x_pos = rect.left() + x_norm * width;
                            painter.line_segment(
                                [
                                    egui::pos2(x_pos, rect.top()),
                                    egui::pos2(x_pos, rect.bottom()),
                                ],
                                grid_stroke,
                            );
                            painter.text(
                                egui::pos2(x_pos, rect.bottom() - 5.0),
                                egui::Align2::CENTER_BOTTOM,
                                format!("{}", x),
                                egui::FontId::monospace(9.0),
                                ZERO_CROSSING_LINE_TEXT_COLOR,
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

                        // --- 2. Graph

                        let mut points = Vec::with_capacity(width as usize);

                        for i in 0..width as usize {
                            let t = 10.0
                                * (i as f32 / width - 0.5)
                                * utils::db_to_gain(params.gain.value())
                                - params.phase.value() / 90.0;

                            let func = match params.waveform.value() {
                                0 => utils::sine(t),
                                1 => utils::triangle(t),
                                2 => utils::saw(t),
                                3 => utils::meander(t),
                                4 => utils::lookup_custom(
                                    samples,
                                    t,
                                    params.interpolation_method.value(),
                                ),
                                _ => utils::sine(t),
                            };

                            let sample = utils::xfader(
                                (i as f32 / width - 0.5) * 2.5,
                                func,
                                params.dw.value(),
                            );

                            let x = rect.left() + i as f32;
                            let y = mid_y - (sample * height_scale);
                            points.push(egui::pos2(x, y));
                        }

                        {
                            let x = rect.left() + rect.width() / 2.0;
                            // Рисуем вертикальную линию
                            painter.line_segment(
                                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                                egui::Stroke::new(1.0, PHASE_LINE_COLOR.linear_multiply(0.5)),
                            );
                        }

                        // --- 3. САМА ЛИНИЯ ГРАФИКА ---
                        painter.add(egui::Shape::line(
                            points,
                            egui::Stroke::new(2.0, GRAPH_LINE_COLOR),
                        ));
                    }

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

                    ui.label("Interpolation Method:");
                    ui.horizontal_wrapped(|ui| {
                        let mut current_wave = params.interpolation_method.value();

                        // Создаем радиокнопки для каждого типа
                        for (val, label) in [(0, "Linear"), (1, "Sine")] {
                            if ui.radio_value(&mut current_wave, val, label).changed() {
                                setter.begin_set_parameter(&params.interpolation_method);
                                setter.set_parameter(&params.interpolation_method, current_wave);
                                setter.end_set_parameter(&params.interpolation_method);
                            }
                        }
                    });
                });
            },
        )
    }
}
