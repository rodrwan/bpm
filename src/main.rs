use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::audio::Signal;
use realfft::RealFftPlanner;
use rustfft::num_complex::Complex32;
use std::fs::File;

fn analyze_bpm(grave_energies: Vec<f32>, sample_rate: u32, hop_size: usize) -> Option<f32> {
    println!("Analizando {} frames de energía", grave_energies.len());
    if grave_energies.len() < 3 {
        println!("No hay suficientes datos para calcular BPM");
        return None;
    }

    // Normalizar las energías
    let max_energy = grave_energies.iter().fold(0.0_f32, |a, &b| a.max(b));
    let normalized_energies: Vec<f32> = grave_energies.iter()
        .map(|&e| e / max_energy)
        .collect();

    // Calcular autocorrelación en el dominio de frames
    let seconds_per_frame = hop_size as f32 / sample_rate as f32;

    // Rango de BPM: 60-180
    let min_bpm = 60.0;
    let max_bpm = 180.0;

    // Convertir BPM a frames
    let min_frames = (60.0 / max_bpm / seconds_per_frame).round() as usize;
    let max_frames = (60.0 / min_bpm / seconds_per_frame).round() as usize;

    let mut autocorr = vec![0.0; max_frames];

    for lag in min_frames..max_frames {
        let mut sum = 0.0;
        let mut count = 0;

        for i in 0..normalized_energies.len() - lag {
            sum += normalized_energies[i] * normalized_energies[i + lag];
            count += 1;
        }

        if count > 0 {
            autocorr[lag] = sum / count as f32;
        }
    }

    // Encontrar picos en la autocorrelación
    let mut peaks = vec![];
    let threshold = 0.05; // Umbral más bajo

    for i in 1..autocorr.len() - 1 {
        if autocorr[i] > threshold
            && autocorr[i] > autocorr[i - 1]
            && autocorr[i] > autocorr[i + 1] {
            peaks.push((i, autocorr[i]));
        }
    }

    // Ordenar picos por magnitud
    peaks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    println!("Encontrados {} picos en autocorrelación", peaks.len());

    // Convertir lags a BPM
    let mut bpm_candidates = vec![];
    for (lag, magnitude) in peaks.iter().take(10) {
        let interval_secs = *lag as f32 * seconds_per_frame;
        let bpm = 60.0 / interval_secs;

        // Solo considerar BPMs en rango musical (60-180)
        if bpm >= 60.0 && bpm <= 180.0 {
            bpm_candidates.push((bpm, *magnitude));
        }
    }

    if bpm_candidates.is_empty() {
        println!("No se encontraron candidatos de BPM válidos");
        return None;
    }

    // Mostrar candidatos
    println!("Candidatos de BPM (top 5):");
    for (i, (bpm, magnitude)) in bpm_candidates.iter().take(5).enumerate() {
        println!("  {}. {:.1} BPM (magnitud: {:.3})", i + 1, bpm, magnitude);
    }

    // Seleccionar el mejor BPM
    let (best_bpm, best_magnitude) = if bpm_candidates.len() >= 2 {
        let (bpm1, mag1) = bpm_candidates[0];
        let (bpm2, mag2) = bpm_candidates[1];

        // Si las magnitudes son similares (diferencia < 10%), preferir el BPM más alto
        if (mag1 - mag2).abs() / mag1 < 0.1 {
            if bpm1 > bpm2 {
                (bpm1, mag1)
            } else {
                (bpm2, mag2)
            }
        } else {
            (bpm1, mag1)
        }
    } else {
        bpm_candidates[0]
    };

    // Redondear a valores musicales típicos
    let rounded_bpm = (best_bpm * 2.0_f32).round() / 2.0;

    println!("BPM seleccionado: {:.1} (magnitud: {:.3})", rounded_bpm, best_magnitude);

    Some(rounded_bpm)
}

fn main() {
    let path = "Electricano - Decisions (Original Mix).aiff";
    let file = File::open(path).unwrap();
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let hint = Hint::new();

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .unwrap();
    let mut format = probed.format;
    let track = format.default_track().unwrap();
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .unwrap();

    let sample_rate = track.codec_params.sample_rate.unwrap();
    let fft_size = 2048;
    let hop_size = fft_size / 2;

    let mut planner = RealFftPlanner::<f32>::new();
    let r2c = planner.plan_fft_forward(fft_size);
    let mut input = r2c.make_input_vec();
    let mut spectrum = r2c.make_output_vec();

    let mut frame = vec![];
    let mut grave_energies = vec![];

    while let Ok(packet) = format.next_packet() {
        let decoded = decoder.decode(&packet).unwrap();
        match decoded {
            symphonia::core::audio::AudioBufferRef::F32(buf) => {
                let num_channels = buf.spec().channels.count();
                for frame_idx in 0..buf.frames() {
                    let mut sample = 0.0;
                    for ch in 0..num_channels {
                        sample += buf.chan(ch)[frame_idx];
                    }
                    sample /= num_channels as f32;
                    frame.push(sample);
                    if frame.len() >= fft_size {
                        input.copy_from_slice(&frame[..fft_size]);
                        r2c.process(&mut input, &mut spectrum).unwrap();
                        let bin_freq = sample_rate as f32 / fft_size as f32;
                        let low_bin = (40.0 / bin_freq).round() as usize;
                        let high_bin = (150.0 / bin_freq).round() as usize;
                        let energy: f32 = spectrum[low_bin..high_bin]
                            .iter()
                            .map(|c: &Complex32| c.norm_sqr())
                            .sum();
                        grave_energies.push(energy);
                        frame.drain(..hop_size);
                    }
                }
            }
            symphonia::core::audio::AudioBufferRef::S16(buf) => {
                let num_channels = buf.spec().channels.count();
                for frame_idx in 0..buf.frames() {
                    let mut sample = 0.0;
                    for ch in 0..num_channels {
                        sample += buf.chan(ch)[frame_idx] as f32 / i16::MAX as f32;
                    }
                    sample /= num_channels as f32;
                    frame.push(sample);
                    if frame.len() >= fft_size {
                        input.copy_from_slice(&frame[..fft_size]);
                        r2c.process(&mut input, &mut spectrum).unwrap();
                        let bin_freq = sample_rate as f32 / fft_size as f32;
                        let low_bin = (40.0 / bin_freq).round() as usize;
                        let high_bin = (150.0 / bin_freq).round() as usize;
                        let energy: f32 = spectrum[low_bin..high_bin]
                            .iter()
                            .map(|c: &Complex32| c.norm_sqr())
                            .sum();
                        grave_energies.push(energy);
                        frame.drain(..hop_size);
                    }
                }
            }
            symphonia::core::audio::AudioBufferRef::U8(buf) => {
                let num_channels = buf.spec().channels.count();
                for frame_idx in 0..buf.frames() {
                    let mut sample = 0.0;
                    for ch in 0..num_channels {
                        sample += (buf.chan(ch)[frame_idx] as f32 - 128.0) / 128.0;
                    }
                    sample /= num_channels as f32;
                    frame.push(sample);
                    if frame.len() >= fft_size {
                        input.copy_from_slice(&frame[..fft_size]);
                        r2c.process(&mut input, &mut spectrum).unwrap();
                        let bin_freq = sample_rate as f32 / fft_size as f32;
                        let low_bin = (40.0 / bin_freq).round() as usize;
                        let high_bin = (150.0 / bin_freq).round() as usize;
                        let energy: f32 = spectrum[low_bin..high_bin]
                            .iter()
                            .map(|c: &Complex32| c.norm_sqr())
                            .sum();
                        grave_energies.push(energy);
                        frame.drain(..hop_size);
                    }
                }
            }
            _ => {
                println!("Formato de buffer no soportado");
                continue;
            }
        }
    }

    // Analizar BPM
    if let Some(bpm) = analyze_bpm(grave_energies, sample_rate, hop_size) {
        println!("BPM estimado: {:.1}", bpm);
    } else {
        println!("No se pudo calcular el BPM");
    }
}
