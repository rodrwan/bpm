use std::env;
use bpm_detector::{BpmDetector, BpmConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_path = env::args().nth(1).expect("No file path given");
    // Uso básico con configuración por defecto
    let detector = BpmDetector::new();

    match detector.detect_from_file(&file_path) {
        Ok(bpm) => println!("BPM detectado: {:.1}", bpm),
        Err(e) => println!("Error: {}", e),
    }

    // Uso con configuración personalizada
    let config = BpmConfig {
        min_frequency: 40.0,  // Frecuencias más graves
        max_frequency: 150.0, // Solo hasta 150 Hz
        min_bpm: 80.0,        // BPM mínimo más alto
        max_bpm: 160.0,       // BPM máximo más bajo
        autocorr_threshold: 0.1, // Umbral más estricto
        ..Default::default()
    };

    let custom_detector = BpmDetector::with_config(config);

    match custom_detector.detect_from_file(&file_path) {
        Ok(bpm) => println!("BPM con configuración personalizada: {:.1}", bpm),
        Err(e) => println!("Error: {}", e),
    }

    Ok(())
}
