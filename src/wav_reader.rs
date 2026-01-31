use nih_plug::nih_log;
use parking_lot::RwLock;
use std::sync::Arc;

pub fn process_wav_from_path(
    path: &str,
    custom_waveform: &Arc<RwLock<Arc<Vec<f32>>>>,
    zero_crossing_points: &Arc<RwLock<Vec<f32>>>,
) {
    if let Ok(reader) = hound::WavReader::open(path) {
        process_wav_reader(reader, custom_waveform, zero_crossing_points);
    } else {
        nih_log!("Failed to open wav at: {}", path);
    }
}

pub fn process_wav_reader(
    mut reader: hound::WavReader<std::io::BufReader<std::fs::File>>,
    custom_waveform: &Arc<RwLock<Arc<Vec<f32>>>>,
    zero_crossing_points: &Arc<RwLock<Vec<f32>>>,
) {
    let mut samples: Vec<f32> = match reader.spec().sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.unwrap_or(0.0)).collect(),
        hound::SampleFormat::Int => reader
            .samples::<i32>()
            .map(|s| s.unwrap_or(0) as f32)
            .collect(),
    };

    // Нормализация
    let max_value = samples.iter().fold(0f32, |acc, s: &f32| acc.max(s.abs()));
    if max_value > 0.0 {
        samples.iter_mut().for_each(|s| *s /= max_value);
    }

    let mut new_zero_crossing_points = Vec::new();
    crate::zero_crossing_detector::zero_crosing_points(&samples, &mut new_zero_crossing_points);

    *custom_waveform.write() = Arc::new(samples);
    *zero_crossing_points.write() = new_zero_crossing_points;
}
