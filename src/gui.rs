use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui, widgets};

//const ZERO_CROSSING_LINE_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 0, 255);
//const PHASE_LINE_TEXT_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 255, 128);
const ZERO_CROSSING_LINE_TEXT_COLOR: egui::Color32 = egui::Color32::from_rgb(127, 255, 127);
const PHASE_LINE_COLOR: egui::Color32 = egui::Color32::from_rgb(127, 255, 127);
const GRAPH_LINE_COLOR: egui::Color32 = egui::Color32::from_rgb(150, 240, 15);
const COMPOSITION_GRAPH_LINE_COLOR: egui::Color32 = egui::Color32::from_rgb(15, 240, 150);
const BG_TOP_COLOR: egui::Color32 = egui::Color32::from_rgb(10, 0, 10);
const BG_MEDIUM_COLOR: egui::Color32 = egui::Color32::from_rgb(5, 5, 5);
const BG_BOTTOM_COLOR: egui::Color32 = egui::Color32::from_rgb(0, 10, 0);
const GRAPH_HEIGHT: f32 = 120.0;

use crate::func::func;
use crate::utils;
use crate::wf_background_task::WFBackgroundTask;
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
                let painter = egui_ctx.layer_painter(egui::LayerId::background());

                let rect = egui_ctx.screen_rect();

                // 1. Создаем пустую сетку (Mesh)
                let mut mesh = egui::Mesh::default();
                mesh.texture_id = egui::TextureId::Managed(0);
                // 2. Добавляем в неё прямоугольник.
                // UV нам не важен, поэтому просто заглушка.
                mesh.add_rect_with_uv(
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(0.0, 0.0)),
                    egui::Color32::WHITE,
                );

                // 3. Красим вершины (0 и 1 — верхние, 2 и 3 — нижние)

                mesh.vertices[0].color = BG_TOP_COLOR; // Лево-верх
                mesh.vertices[1].color = BG_MEDIUM_COLOR; // Право-верх
                mesh.vertices[2].color = BG_BOTTOM_COLOR; // Право-низ
                mesh.vertices[3].color = BG_MEDIUM_COLOR; // Лево-низ

                // Отправляем на отрисовку
                painter.add(egui::Shape::mesh(mesh));

                let table_guard = waveform_arc.read();
                let samples = &**table_guard;

                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show(egui_ctx, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.label(
                                egui::RichText::new("WAVEFOLDER DISTORTION")
                                    .italics()
                                    .size(20.0),
                            );
                        });

                        ui.add_space(15.0);

                        let available_width = ui.available_width();

                        // график функции
                        ui.group(|ui| {
                            ui.vertical_centered(|ui| {
                                ui.label("graph of the function");
                                let (rect, response) = ui.allocate_at_least(
                                    egui::vec2(available_width, GRAPH_HEIGHT),
                                    egui::Sense::drag(),
                                );

                                if response.dragged() {
                                    let delta_x = response.drag_delta().x;

                                    let phase_delta = (delta_x / rect.width())
                                        * utils::db_to_gain(params.gain.value()).max(4.0)
                                        * 12.0;

                                    let mut new_phase = params.phase.value() - phase_delta;

                                    new_phase = new_phase.rem_euclid(360.0);

                                    setter.begin_set_parameter(&params.phase);
                                    setter.set_parameter(&params.phase, new_phase);
                                    setter.end_set_parameter(&params.phase);

                                    let delta_y = response.drag_delta().y;
                                    let delta_bias = -delta_y / rect.width();
                                    let new_bias =
                                        (params.bias.value() + delta_bias).clamp(-1.0, 1.0);
                                    setter.begin_set_parameter(&params.bias);
                                    setter.set_parameter(&params.bias, new_bias);
                                    setter.end_set_parameter(&params.bias);
                                }

                                if response.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                                    let scroll_delta = -ui
                                        .input(|i| i.smooth_scroll_delta.y + i.raw_scroll_delta.y);
                                    let sensitivity =
                                        if ui.input(|i| i.modifiers.command || i.modifiers.ctrl) {
                                            0.001
                                        } else {
                                            0.1
                                        };
                                    use crate::wf_params::{MAX_GAIN, MIN_GAIN};
                                    let new_gain = (params.gain.value()
                                        + scroll_delta * sensitivity)
                                        .clamp(MIN_GAIN, MAX_GAIN);
                                    setter.begin_set_parameter(&params.gain);
                                    setter.set_parameter(&params.gain, new_gain);
                                    setter.end_set_parameter(&params.gain);
                                }

                                let painter = ui.painter_at(rect);
                                painter.rect_filled(rect, 4.0, egui::Color32::from_black_alpha(30));

                                if !samples.is_empty() {
                                    let mid_y = rect.center().y;
                                    let height_scale = rect.height() * 0.4;
                                    let width = rect.width();

                                    // отрисовка линий
                                    let grid_stroke =
                                        egui::Stroke::new(0.5, egui::Color32::from_white_alpha(50));
                                    for x in [
                                        -1.25, -1.0, -0.75, -0.5, -0.25, 0.25, 0.5, 0.75, 1.0, 1.25,
                                    ] {
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
                                    painter.line_segment(
                                        [
                                            egui::pos2(rect.left(), mid_y),
                                            egui::pos2(rect.right(), mid_y),
                                        ],
                                        grid_stroke,
                                    );

                                    // отрисовка точек графика

                                    let mut points = Vec::with_capacity(width as usize);

                                    for i in 0..width as usize {
                                        let f_i = func(
                                            i as f32 / width - 0.5,
                                            params.waveform.value(),
                                            params.interpolation_method.value(),
                                            params.gain.value(),
                                            params.phase.value() / 90.0,
                                            params.func_gain.value(),
                                            params.bias.value(),
                                            samples,
                                        );

                                        let x = rect.left() + i as f32;
                                        let y = mid_y - (f_i * height_scale);
                                        points.push(egui::pos2(x, y));
                                    }

                                    // Линия по середине
                                    {
                                        let x = rect.left() + rect.width() / 2.0;
                                        painter.line_segment(
                                            [
                                                egui::pos2(x, rect.top()),
                                                egui::pos2(x, rect.bottom()),
                                            ],
                                            egui::Stroke::new(
                                                1.0,
                                                PHASE_LINE_COLOR.linear_multiply(0.5),
                                            ),
                                        );
                                    }

                                    painter.add(egui::Shape::line(
                                        points,
                                        egui::Stroke::new(2.0, GRAPH_LINE_COLOR),
                                    ));
                                }
                            })
                        });
                        ui.add_space(10.0);

                        {
                            //графики функции к синусоиде([-pi;pi]) и линейной функции ([-1;1])
                            ui.columns(2, |cols| {
                                // Первая колонка
                                cols[0].vertical_centered(|ui| {
                                    ui.group(|ui| {
                                        ui.label("composition to sine");
                                        let available_width = ui.available_width();
                                        let (rect, _response) = ui.allocate_at_least(
                                            egui::vec2(available_width, GRAPH_HEIGHT),
                                            egui::Sense::focusable_noninteractive(),
                                        );

                                        let mid_y = rect.center().y;
                                        let height_scale = rect.height() * 0.4;
                                        let width = rect.width();

                                        let painter = ui.painter_at(rect);
                                        painter.rect_filled(
                                            rect,
                                            4.0,
                                            egui::Color32::from_black_alpha(30),
                                        );
                                        let mut points = vec![];

                                        for i in 0..width as usize {
                                            let x = (2.0
                                                * std::f32::consts::PI
                                                * (i as f32 / width - 0.5))
                                                .sin();

                                            let f_i = func(
                                                x,
                                                params.waveform.value(),
                                                params.interpolation_method.value(),
                                                params.gain.value(),
                                                params.phase.value() / 4.0,
                                                params.func_gain.value(),
                                                params.bias.value(),
                                                samples,
                                            );

                                            let sample = if params.clipping_enable.value() {
                                                utils::xfader(
                                                    x,
                                                    f_i,
                                                    params.dw.value().clamp(-1.0, 1.0),
                                                )
                                            } else {
                                                utils::xfader(x, f_i, params.dw.value())
                                            };

                                            let x = rect.left() + i as f32;
                                            let y = mid_y - (sample * height_scale * 0.9);
                                            points.push(egui::pos2(x, y));
                                        }

                                        painter.add(egui::Shape::line(
                                            points,
                                            egui::Stroke::new(2.0, COMPOSITION_GRAPH_LINE_COLOR),
                                        ));
                                    });
                                });

                                // Вторая колонка
                                cols[1].vertical_centered(|ui| {
                                    ui.group(|ui| {
                                        ui.label("composition to linear function");
                                        let available_width = ui.available_width();

                                        let (rect, _response) = ui.allocate_at_least(
                                            egui::vec2(available_width, GRAPH_HEIGHT),
                                            egui::Sense::focusable_noninteractive(),
                                        );

                                        let mid_y = rect.center().y;
                                        let height_scale = rect.height() * 0.4;
                                        let width = rect.width();

                                        let painter = ui.painter_at(rect);
                                        painter.rect_filled(
                                            rect,
                                            4.0,
                                            egui::Color32::from_black_alpha(30),
                                        );
                                        let mut points = vec![];
                                        for i in 0..width as usize {
                                            let x = 2.0 * i as f32 / width - 1.0;
                                            let f_i = func(
                                                x,
                                                params.waveform.value(),
                                                params.interpolation_method.value(),
                                                params.gain.value(),
                                                params.phase.value() / 4.0,
                                                params.func_gain.value(),
                                                params.bias.value(),
                                                samples,
                                            );

                                            let sample = if params.clipping_enable.value() {
                                                utils::xfader(
                                                    x,
                                                    f_i,
                                                    params.dw.value().clamp(-1.0, 1.0),
                                                )
                                            } else {
                                                utils::xfader(x, f_i, params.dw.value())
                                            };

                                            let x = rect.left() + i as f32;
                                            let y = mid_y - (sample * height_scale * 0.9);
                                            points.push(egui::pos2(x, y));
                                        }

                                        painter.add(egui::Shape::line(
                                            points,
                                            egui::Stroke::new(2.0, COMPOSITION_GRAPH_LINE_COLOR),
                                        ));
                                    });
                                });
                            });
                        }

                        ui.add_space(10.0);

                        // Слайдеры
                        let slider_width = ui.available_width() / 4.0;

                        ui.columns(2, |cols| {
                            cols[0].vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Dry/Wet"));
                                    ui.add(
                                        widgets::ParamSlider::for_param(&params.dw, setter)
                                            .with_width(slider_width),
                                    );
                                });
                                ui.add_space(5.0);

                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Drive"));
                                    ui.add(
                                        widgets::ParamSlider::for_param(&params.gain, setter)
                                            .with_width(slider_width),
                                    );
                                });

                                ui.add_space(5.0);

                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Phase"));
                                    ui.add(
                                        widgets::ParamSlider::for_param(&params.phase, setter)
                                            .with_width(slider_width),
                                    );
                                });
                            });
                            cols[1].vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Func gain"));
                                    ui.add(
                                        widgets::ParamSlider::for_param(&params.func_gain, setter)
                                            .with_width(slider_width),
                                    );
                                });
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Bias"));
                                    ui.add(
                                        widgets::ParamSlider::for_param(&params.bias, setter)
                                            .with_width(slider_width),
                                    );
                                });
                                ui.horizontal(|ui| {
                                    let mut current_clipping_state = params.clipping_enable.value();
                                    if ui
                                        .checkbox(
                                            &mut current_clipping_state,
                                            egui::RichText::new("Clipping enable"),
                                        )
                                        .changed()
                                    {
                                        setter.begin_set_parameter(&params.clipping_enable);
                                        setter.set_parameter(
                                            &params.clipping_enable,
                                            current_clipping_state,
                                        );
                                        setter.end_set_parameter(&params.clipping_enable);
                                    }
                                });
                            });
                        });

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

                        ui.horizontal_wrapped(|ui| {
                            ui.group(|ui| {
                                ui.label("Interpolation Method:");
                                let mut current_wave = params.interpolation_method.value();

                                // Создаем радиокнопки для каждого типа
                                for (val, label) in [(0, "Linear"), (1, "Sine")] {
                                    if ui.radio_value(&mut current_wave, val, label).changed() {
                                        setter.begin_set_parameter(&params.interpolation_method);
                                        setter.set_parameter(
                                            &params.interpolation_method,
                                            current_wave,
                                        );
                                        setter.end_set_parameter(&params.interpolation_method);
                                    }
                                }
                            });
                        });
                    });
            },
        )
    }
}
