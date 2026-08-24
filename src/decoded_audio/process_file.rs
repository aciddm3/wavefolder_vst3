use crate::decoded_audio::{load_file::load_file, DecodedAudio};
use parking_lot::RwLock;
use std::{path::Path, sync::Arc};

pub fn process_file_from_path(
    path_str: &String,
    custom_waveform: &Arc<RwLock<DecodedAudio>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let decoded_audio = load_file(Path::new(path_str))?;
    let custom_waveform_writer = &mut *custom_waveform.write();
    *custom_waveform_writer = decoded_audio;
    Ok(())
}
