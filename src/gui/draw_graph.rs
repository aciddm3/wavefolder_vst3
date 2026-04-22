use std::sync::Arc;

use nih_plug::prelude::ParamSetter;
use nih_plug_egui::egui;

use crate::{utils, wf_params::WFParams};

const ZERO_CROSSING_LINE_TEXT_COLOR: egui::Color32 = egui::Color32::from_rgb(127, 255, 127);
const PHASE_LINE_COLOR: egui::Color32 = egui::Color32::from_rgb(127, 255, 127);
const GRAPH_LINE_COLOR: egui::Color32 = egui::Color32::from_rgb(150, 240, 15);
const COMPOSITION_GRAPH_LINE_COLOR: egui::Color32 = egui::Color32::from_rgb(15, 240, 150);
const GRAPH_HEIGHT: f32 = 120.0;

#[inline]
pub fn draw_graph(
    ui: &mut egui::Ui,
    samples: &Vec<f32>,
    setter: &ParamSetter,
    params: Arc<WFParams>,
	
) {
    let (rect, response) = ui.allocate_at_least(
        egui::vec2(ui.available_width(), GRAPH_HEIGHT),
        egui::Sense::drag(),
    );

    if response.dragged() {
        let delta_x = response.drag_delta().x;

        let phase_delta =
            (delta_x / rect.width()) * utils::db_to_gain(params.gain.value()).max(4.0) * 12.0;

        let mut new_phase = params.phase.value() - phase_delta;

        new_phase = new_phase.rem_euclid(360.0);

        setter.begin_set_parameter(&params.phase);
        setter.set_parameter(&params.phase, new_phase);
        setter.end_set_parameter(&params.phase);

        let delta_y = response.drag_delta().y;
        let delta_bias = -delta_y / rect.width();
        let new_bias = (params.bias.value() + delta_bias).clamp(-1.0, 1.0);
        setter.begin_set_parameter(&params.bias);
        setter.set_parameter(&params.bias, new_bias);
        setter.end_set_parameter(&params.bias);
    }

    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        let scroll_delta = -ui.input(|i| i.smooth_scroll_delta.y + i.raw_scroll_delta.y);
        let sensitivity = if ui.input(|i| i.modifiers.command || i.modifiers.ctrl) {
            0.001
        } else {
            0.1
        };
        use crate::wf_params::{MAX_GAIN, MIN_GAIN};
        let new_gain = (params.gain.value() + scroll_delta * sensitivity).clamp(MIN_GAIN, MAX_GAIN);
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
        let grid_stroke = egui::Stroke::new(0.5, egui::Color32::from_white_alpha(50));
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
            let f_i = crate::func::func(
                (i as f32 / width - 0.5) * 2.5,
                params.waveform.value(),
                params.interpolation_method.value(),
                params.gain.value(),
                params.phase.value(),
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
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                egui::Stroke::new(1.0, PHASE_LINE_COLOR.linear_multiply(0.5)),
            );
        }

        painter.add(egui::Shape::line(
            points,
            egui::Stroke::new(2.0, GRAPH_LINE_COLOR),
        ));
    };
}

#[inline]
pub fn draw_graph_composition(
    ui: &mut egui::Ui,
    samples: &Vec<f32>,
    params: Arc<WFParams>,
	comp_func : impl Fn(f32) -> f32,
) {
    let available_width = ui.available_width();
    let (rect, _response) = ui.allocate_at_least(
        egui::vec2(available_width, GRAPH_HEIGHT),
        egui::Sense::focusable_noninteractive(),
    );

    let mid_y = rect.center().y;
    let height_scale = rect.height() * 0.4;
    let width = rect.width();

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, egui::Color32::from_black_alpha(30));
    let mut points = vec![];

    for i in 0..=width as usize {
        let x = comp_func(2.0 * (i as f32 / width - 0.5));

        let f_i = crate::func::func(
            x,
            params.waveform.value(),
            params.interpolation_method.value(),
            params.gain.value(),
            params.phase.value(),
            params.func_gain.value(),
            params.bias.value(),
            samples,
        );

        let sample = if params.clipping_enable.value() {
            utils::xfader(x, f_i, params.dw.value().clamp(-1.0, 1.0))
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
}
