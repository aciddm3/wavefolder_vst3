use std::path::Path;
use crate::decoded_audio::DecodedAudio;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub fn load_file(path: &Path) -> Result<DecodedAudio, Box<dyn std::error::Error>> {
    let src = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;

    let mut format = probed.format;
    let track = format.default_track().ok_or("no default track found")?;
	let track_id = track.id;
    let channels = track.codec_params.channels.ok_or("channels not found")?;
    let channel_count = channels.count();

    let mut decoder = symphonia::default::get_codecs().make(
        &track.codec_params,
        &DecoderOptions::default(),
    )?;

    let mut all_samples: Vec<Vec<f32>> = vec![Vec::new(); channel_count];

    if let Some(n_frames) = track.codec_params.n_frames {
        for ch in &mut all_samples {
            ch.reserve(n_frames as usize);
        }
    }

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(Box::new(e)),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(Error::DecodeError(_)) => continue,
            Err(e) => return Err(Box::new(e)),
        };

        let spec = *decoded.spec();
        let frames = decoded.frames();

        let mut sample_buf = SampleBuffer::<f32>::new(frames as u64, spec);
        sample_buf.copy_interleaved_ref(decoded);
        let samples = sample_buf.samples();

        for (i, &sample) in samples.iter().enumerate() {
            all_samples[i % channel_count].push(sample);
        }
    }

    Ok(DecodedAudio::new(all_samples))
}