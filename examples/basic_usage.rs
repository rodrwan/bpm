use std::env;
use bpm_detector::{BpmDetector, BpmConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_path = env::args().nth(1).expect("No file path given");
    // Basic usage with default configuration
    let detector = BpmDetector::new();

    match detector.detect_from_file(&file_path) {
        Ok(bpm) => println!("Detected BPM: {:.1}", bpm),
        Err(e) => println!("Error: {}", e),
    }

    // Usage with custom configuration
    let config = BpmConfig {
        min_frequency: 40.0,  // Lower frequencies
        max_frequency: 150.0, // Only up to 150 Hz
        min_bpm: 80.0,        // Higher minimum BPM
        max_bpm: 160.0,       // Lower maximum BPM
        autocorr_threshold: 0.1, // Stricter threshold
        ..Default::default()
    };

    let custom_detector = BpmDetector::with_config(config);

    match custom_detector.detect_from_file(&file_path) {
        Ok(bpm) => println!("BPM with custom configuration: {:.1}", bpm),
        Err(e) => println!("Error: {}", e),
    }

    Ok(())
}
