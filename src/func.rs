use crate::utils;

#[inline]
pub fn func(
    input: f32,
    waveform: i32,
    interpolation_method: i32,
    drive: f32,
    phase: f32,
    func_gain: f32,
    bias: f32,
    custom_table: &[f32],
) -> f32 {
    let input_folded = input * utils::db_to_gain(drive) + phase / 90.0;
    (match waveform {
        0 => utils::sine(input_folded),
        1 => utils::triangle(input_folded),
        2 => utils::saw(input_folded),
        3 => utils::meander(input_folded),
        4 => utils::lookup_custom(custom_table, input_folded, interpolation_method),
        _ => utils::sine(input_folded),
    }) * utils::db_to_gain(func_gain)
        + bias
}
