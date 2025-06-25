use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::audio::Signal;
use realfft::RealFftPlanner;
use rustfft::num_complex::Complex32;
use std::fs::File;

const FFT_SIZE: usize = 2048;
const HOP_SIZE: usize = FFT_SIZE / 2;
const MIN_BPM: f32 = 60.0;
const MAX_BPM: f32 = 180.0;
const MIN_FREQUENCY: f32 = 50.0;
const MAX_FREQUENCY: f32 = 1000.0;
const AUTOCORR_THRESHOLD: f32 = 0.05;

pub struct BpmConfig {
    pub fft_size: usize,
    pub hop_size: usize,
    pub min_frequency: f32,
    pub max_frequency: f32,
    pub min_bpm: f32,
    pub max_bpm: f32,
    pub autocorr_threshold: f32,
}

impl Default for BpmConfig {
    fn default() -> Self {
        Self {
            fft_size: FFT_SIZE,
            hop_size: HOP_SIZE,
            min_frequency: MIN_FREQUENCY,
            max_frequency: MAX_FREQUENCY,
            min_bpm: MIN_BPM,
            max_bpm: MAX_BPM,
            autocorr_threshold: AUTOCORR_THRESHOLD,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BpmError {
    #[error("File not found: {0}")]
    FileNotFound(String),
    #[error("Unsupported audio format")]
    UnsupportedFormat,
    #[error("Insufficient data for BPM detection")]
    InsufficientData,
    #[error("No valid BPM found in range {min}-{max}")]
    NoValidBpm { min: f32, max: f32 },
}

pub struct BpmDetector {
    config: BpmConfig,
}

impl BpmDetector {
    pub fn new() -> Self {
        Self {
            config: BpmConfig::default(),
        }
    }

    pub fn with_config(config: BpmConfig) -> Self {
        Self { config }
    }
}

impl Default for BpmDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl BpmDetector {
    pub fn detect_from_file(&self, path: &str) -> Result<f32, BpmError> {
        let file = File::open(path).map_err(|_| BpmError::FileNotFound(path.to_string()))?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        // Configurar hint con la extensión del archivo para mejor detección de formato
        let mut hint = Hint::new();
        if let Some(extension) = std::path::Path::new(path).extension() {
            if let Some(ext_str) = extension.to_str() {
                hint.with_extension(ext_str);
            }
        }

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
            .map_err(|_| BpmError::UnsupportedFormat)?;

        let mut format = probed.format;
        let track = format.default_track().ok_or(BpmError::UnsupportedFormat)?;
        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(|_| BpmError::UnsupportedFormat)?;

        let sample_rate = track.codec_params.sample_rate.ok_or(BpmError::UnsupportedFormat)?;
        let mut planner = RealFftPlanner::<f32>::new();
        let r2c = planner.plan_fft_forward(self.config.fft_size);
        let mut input = r2c.make_input_vec();
        let mut spectrum = r2c.make_output_vec();

        let mut frame = vec![];
        let mut energies = vec![];

        while let Ok(packet) = format.next_packet() {
            let decoded = decoder.decode(&packet).map_err(|_| BpmError::UnsupportedFormat)?;
            match decoded {
                symphonia::core::audio::AudioBufferRef::F32(buf) => {
                    for frame_idx in 0..buf.frames() {
                        let sample = buf.chan(0)[frame_idx];
                        frame.push(sample);
                        if frame.len() >= self.config.fft_size {
                            input.copy_from_slice(&frame[..self.config.fft_size]);
                            r2c.process(&mut input, &mut spectrum).map_err(|_| BpmError::UnsupportedFormat)?;

                            let bin_freq = sample_rate as f32 / self.config.fft_size as f32;
                            let low_bin = (self.config.min_frequency / bin_freq).round() as usize;
                            let high_bin = (self.config.max_frequency / bin_freq).round() as usize;

                            let energy: f32 = spectrum[low_bin..high_bin]
                                .iter()
                                .map(|c: &Complex32| c.norm_sqr())
                                .sum();

                            energies.push(energy);
                            frame.drain(..self.config.hop_size);
                        }
                    }
                }
                symphonia::core::audio::AudioBufferRef::S16(buf) => {
                    for frame_idx in 0..buf.frames() {
                        let sample = buf.chan(0)[frame_idx] as f32 / i16::MAX as f32;
                        frame.push(sample);
                        if frame.len() >= self.config.fft_size {
                            input.copy_from_slice(&frame[..self.config.fft_size]);
                            r2c.process(&mut input, &mut spectrum).map_err(|_| BpmError::UnsupportedFormat)?;

                            let bin_freq = sample_rate as f32 / self.config.fft_size as f32;
                            let low_bin = (self.config.min_frequency / bin_freq).round() as usize;
                            let high_bin = (self.config.max_frequency / bin_freq).round() as usize;

                            let energy: f32 = spectrum[low_bin..high_bin]
                                .iter()
                                .map(|c: &Complex32| c.norm_sqr())
                                .sum();

                            energies.push(energy);
                            frame.drain(..self.config.hop_size);
                        }
                    }
                }
                symphonia::core::audio::AudioBufferRef::U8(buf) => {
                    for frame_idx in 0..buf.frames() {
                        let sample = (buf.chan(0)[frame_idx] as f32 - 128.0) / 128.0;
                        frame.push(sample);
                        if frame.len() >= self.config.fft_size {
                            input.copy_from_slice(&frame[..self.config.fft_size]);
                            r2c.process(&mut input, &mut spectrum).map_err(|_| BpmError::UnsupportedFormat)?;

                            let bin_freq = sample_rate as f32 / self.config.fft_size as f32;
                            let low_bin = (self.config.min_frequency / bin_freq).round() as usize;
                            let high_bin = (self.config.max_frequency / bin_freq).round() as usize;

                            let energy: f32 = spectrum[low_bin..high_bin]
                                .iter()
                                .map(|c: &Complex32| c.norm_sqr())
                                .sum();

                            energies.push(energy);
                            frame.drain(..self.config.hop_size);
                        }
                    }
                }
                _ => continue,
            }
        }

        self.detect_from_samples(&energies, sample_rate)
    }

    pub fn detect_from_samples(&self, energies: &[f32], sample_rate: u32) -> Result<f32, BpmError> {
        if energies.len() < 3 {
            return Err(BpmError::InsufficientData);
        }

        // Normalizar energías
        let max_energy = energies.iter().fold(0.0_f32, |a, &b| a.max(b));
        let normalized: Vec<f32> = energies.iter().map(|&e| e / max_energy).collect();

        // Calcular autocorrelación
        let seconds_per_frame = self.config.hop_size as f32 / sample_rate as f32;
        let min_frames = (self.config.min_bpm / self.config.max_bpm / seconds_per_frame).round() as usize;
        let max_frames = (self.config.max_bpm / self.config.min_bpm / seconds_per_frame).round() as usize;

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
            if autocorr[i] > self.config.autocorr_threshold
                && autocorr[i] > autocorr[i - 1]
                && autocorr[i] > autocorr[i + 1] {
                peaks.push((i, autocorr[i]));
            }
        }

        peaks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Convertir a BPM
        let mut candidates = vec![];
        for (lag, magnitude) in peaks.iter().take(5) {
            let interval = *lag as f32 * seconds_per_frame;
            let bpm = self.config.min_bpm / interval;
            if bpm >= self.config.min_bpm && bpm <= self.config.max_bpm {
                candidates.push((bpm, *magnitude));
            }
        }

        if candidates.is_empty() {
            return Err(BpmError::NoValidBpm {
                min: self.config.min_bpm,
                max: self.config.max_bpm
            });
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

        Ok((best_bpm * 2.0).round() / 2.0)
    }
}
