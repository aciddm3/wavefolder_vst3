use std::sync::Arc;
use nih_plug::nih_log;
use parking_lot::RwLock;

pub fn process_wav_from_path(path: &str, custom_waveform: &Arc<RwLock<Arc<Vec<f32>>>>) {
    if let Ok(reader) = hound::WavReader::open(path) {
       	process_wav_reader(reader, custom_waveform);
    } else {
        nih_log!("Failed to open wav at: {}", path);
    }
}

pub fn process_wav_reader(mut reader: hound::WavReader<std::io::BufReader<std::fs::File>>, custom_waveform: &Arc<RwLock<Arc<Vec<f32>>>>) {
    let mut samples: Vec<f32> = match reader.spec().sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.unwrap_or(0.0)).collect(),
        hound::SampleFormat::Int => reader.samples::<i32>().map(|s| s.unwrap_or(0) as f32).collect(),
    };

    // Нормализация
    let max_value = samples.iter().fold(0f32, |acc, s: &f32| acc.max(s.abs()));
    if max_value > 0.0 {
        samples.iter_mut().for_each(|s| *s /= max_value);
    }
    
    *custom_waveform.write() = Arc::new(samples);
}