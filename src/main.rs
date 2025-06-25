use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::audio::Signal;
use realfft::RealFftPlanner;
use rustfft::num_complex::Complex32;
use std::fs::File;

fn analyze_bpm(energies: Vec<f32>, sample_rate: u32, hop_size: usize) -> Option<f32> {
    if energies.len() < 3 {
        return None;
    }

    // Normalizar energías
    let max_energy = energies.iter().fold(0.0_f32, |a, &b| a.max(b));
    let normalized: Vec<f32> = energies.iter().map(|&e| e / max_energy).collect();

    // Calcular autocorrelación
    let seconds_per_frame = hop_size as f32 / sample_rate as f32;
    let min_frames = (60.0 / 180.0 / seconds_per_frame).round() as usize; // 180 BPM
    let max_frames = (60.0 / 60.0 / seconds_per_frame).round() as usize;  // 60 BPM

    let mut autocorr = vec![0.0; max_frames];

    for lag in min_frames..max_frames {
        let mut sum = 0.0;
        let mut count = 0;

        for i in 0..normalized.len() - lag {
            sum += normalized[i] * normalized[i + lag];
            count += 1;
        }

        if count > 0 {
            autocorr[lag] = sum / count as f32;
        }
    }

    // Encontrar picos
    let mut peaks = vec![];
    for i in 1..autocorr.len() - 1 {
        if autocorr[i] > 0.05 && autocorr[i] > autocorr[i - 1] && autocorr[i] > autocorr[i + 1] {
            peaks.push((i, autocorr[i]));
        }
    }

    peaks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // Convertir a BPM
    let mut candidates = vec![];
    for (lag, magnitude) in peaks.iter().take(5) {
        let interval = *lag as f32 * seconds_per_frame;
        let bpm = 60.0 / interval;
        if bpm >= 60.0 && bpm <= 180.0 {
            candidates.push((bpm, *magnitude));
        }
    }

    if candidates.is_empty() {
        return None;
    }

    // Seleccionar mejor BPM (preferir más alto si magnitudes similares)
    let (bpm1, mag1) = candidates[0];
    let (best_bpm, _) = if candidates.len() >= 2 {
        let (bpm2, mag2) = candidates[1];
        if (mag1 - mag2).abs() / mag1 < 0.1 && bpm2 > bpm1 {
            (bpm2, mag2)
        } else {
            (bpm1, mag1)
        }
    } else {
        (bpm1, mag1)
    };

    Some((best_bpm * 2.0).round() / 2.0)
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
    let mut energies = vec![];

    while let Ok(packet) = format.next_packet() {
        let decoded = decoder.decode(&packet).unwrap();
        match decoded {
            symphonia::core::audio::AudioBufferRef::F32(buf) => {
                for frame_idx in 0..buf.frames() {
                    let sample = buf.chan(0)[frame_idx];
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

                        energies.push(energy);
                        frame.drain(..hop_size);
                    }
                }
            }
            symphonia::core::audio::AudioBufferRef::S16(buf) => {
                for frame_idx in 0..buf.frames() {
                    let sample = buf.chan(0)[frame_idx] as f32 / i16::MAX as f32;
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

                        energies.push(energy);
                        frame.drain(..hop_size);
                    }
                }
            }
            symphonia::core::audio::AudioBufferRef::U8(buf) => {
                for frame_idx in 0..buf.frames() {
                    let sample = (buf.chan(0)[frame_idx] as f32 - 128.0) / 128.0;
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

                        energies.push(energy);
                        frame.drain(..hop_size);
                    }
                }
            }
            _ => continue,
        }
    }

    if let Some(bpm) = analyze_bpm(energies, sample_rate, hop_size) {
        println!("BPM estimado: {:.1}", bpm);
    } else {
        println!("No se pudo calcular el BPM");
    }
}
