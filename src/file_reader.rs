use parking_lot::RwLock;
use std::sync::Arc;
use rodio::Decoder;
use std::fs::File;
use std::io::BufReader;


pub fn process_file_from_path(
    path: &str,
    custom_waveform: &Arc<RwLock<Arc<Vec<f32>>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let source = Decoder::new(BufReader::new(file))?;
    
    // Samples collect
    let mut samples: Vec<f32> = source.collect();

    // Normalization
    let max_value = samples.iter().fold(0f32, |acc, s: &f32| acc.max(s.abs()));
    if max_value > 0.0 {
        samples.iter_mut().for_each(|s| *s /= max_value);
    }

    // writing
    *custom_waveform.write() = Arc::new(samples);
    Ok(())
}
