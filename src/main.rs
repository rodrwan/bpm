use std::env;
use bpm_detector::{BpmDetector, BpmConfig};

const FFT_SIZE: usize = 2048;
const HOP_SIZE: usize = FFT_SIZE / 2;
const MIN_BPM: f32 = 60.0;
const MAX_BPM: f32 = 180.0;
const MIN_FREQUENCY: f32 = 50.0;
const MAX_FREQUENCY: f32 = 1000.0;
const AUTOCORR_THRESHOLD: f32 = 0.05;

fn main() {
    let file_path = env::args().nth(1).expect("No file path given");

    // Personalized configuration
    let config = BpmConfig {
        fft_size: FFT_SIZE,
        hop_size: HOP_SIZE,
        min_frequency: MIN_FREQUENCY,
        max_frequency: MAX_FREQUENCY,
        min_bpm: MIN_BPM,
        max_bpm: MAX_BPM,
        autocorr_threshold: AUTOCORR_THRESHOLD,
        ..Default::default()
    };

    let custom_detector = BpmDetector::with_config(config);

    match custom_detector.detect_from_file(&file_path) {
        Ok(bpm) => println!("Estimated BPM: {:.1}", bpm),
        Err(e) => println!("Error: {}", e),
    }
}
