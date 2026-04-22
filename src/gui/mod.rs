use nih_plug::prelude::*;
use nih_plug_egui::egui::RichText;
use nih_plug_egui::{create_egui_editor, egui, widgets};

const BG_TOP_COLOR: egui::Color32 = egui::Color32::from_rgb(10, 0, 10);
const BG_MEDIUM_COLOR: egui::Color32 = egui::Color32::from_rgb(5, 5, 5);
const BG_BOTTOM_COLOR: egui::Color32 = egui::Color32::from_rgb(0, 10, 0);

use crate::func::func;
use crate::gui::draw_graph::{draw_graph, draw_graph_composition};
use crate::wf_background_task::WFBackgroundTask;
use crate::wf_struct::WF;

mod draw_graph;

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

                        // график функции
                        ui.group(|ui| {
                            ui.label("graph of the function");
                            draw_graph(ui, samples, setter, params.clone());
                        });
                        ui.add_space(10.0);

                        //графики функции к синусоиде([-pi;pi]) и линейной функции ([-1;1])
                        ui.columns(2, |cols| {
                            // Первая колонка
                            cols[0].vertical_centered(|ui| {
                                ui.group(|ui| {
                                    ui.label("composition to sine");
                                    draw_graph_composition(ui, samples, params.clone(), |x| {
                                        (x * std::f32::consts::PI).sin()
                                    })
                                });
                            });

                            // Вторая колонка
                            cols[1].vertical_centered(|ui| {
                                ui.group(|ui| {
                                    ui.label("composition to linear function");
                                    draw_graph_composition(ui, samples, params.clone(), |x| x)
                                });
                            });
                        });

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
                                    if ui.button(RichText::new("Snap bias to zero")).clicked() {
                                        let value_to_set = -func(
                                            0.0,
                                            params.waveform.value(),
                                            params.interpolation_method.value(),
                                            params.gain.value(),
                                            params.phase.value(),
                                            params.func_gain.value(),
                                            0.0,
                                            samples,
                                        );
                                        setter.begin_set_parameter(&params.bias);
                                        setter.set_parameter(&params.bias, value_to_set);
                                        setter.end_set_parameter(&params.bias);
                                    }
                                });
                            });
                        });

                        ui.horizontal(|ui| {
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
                                        (4, "From file"),
                                    ] {
                                        if ui.radio_value(&mut current_wave, val, label).changed() {
                                            setter.begin_set_parameter(&params.waveform);
                                            setter.set_parameter(&params.waveform, current_wave);
                                            setter.end_set_parameter(&params.waveform);
                                        }
                                    }
                                });
                            });

                            ui.horizontal(|ui| {
                                if ui.button("Load file").clicked() {
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
