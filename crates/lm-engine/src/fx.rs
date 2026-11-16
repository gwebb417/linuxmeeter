//! The reverb's impulse response, generated rather than shipped.
//!
//! PipeWire's builtin convolver plays any impulse response from a file, so the
//! reverb is a synthesized tail: decaying noise, darker as it fades (high
//! frequencies die first in real rooms), with independent noise per channel
//! for width. Decay and damping are the two user settings; each combination
//! is written once to the cache directory and reused.

use std::fs;
use std::io;
use std::path::PathBuf;

/// Sample rate of the generated file. The convolver resamples if the graph
/// runs at another rate.
pub const IR_RATE: u32 = 48000;

/// Stereo impulse response for an RT60 of `decay_ms` (the tail falls 60 dB
/// over that time) with `damping` 0..1 (0 = bright throughout, 1 = the tail
/// ends dark). Each channel is scaled to unit energy so a reverb send level
/// means roughly the same loudness at any decay, then to 0.7 for headroom.
pub fn reverb_ir(decay_ms: f32, damping: f32) -> Vec<[f32; 2]> {
    let decay_s = (decay_ms / 1000.0).clamp(0.05, 10.0);
    let damping = damping.clamp(0.0, 1.0);
    let n = ((decay_s * IR_RATE as f32) as usize).max(1);
    // -60 dB at n: exp(-k) = 1e-3.
    let k = 6.907_755_f32 / n as f32;
    let mut rng = [0x9E37_79B9_7F4A_7C15_u64, 0xD1B5_4A32_D192_ED03_u64];
    let mut lp = [0.0f32; 2];
    let mut out = vec![[0.0f32; 2]; n];
    for (i, frame) in out.iter_mut().enumerate() {
        let t = i as f32 / n as f32;
        let env = (-k * i as f32).exp();
        // One-pole low-pass whose cutoff falls over the tail (squared, so the
        // highs roll off audibly, as air and soft surfaces do in a real room).
        let a = (1.0 - damping * 0.97 * t).powi(2);
        for ch in 0..2 {
            let x = noise(&mut rng[ch]);
            lp[ch] += a * (x - lp[ch]);
            frame[ch] = lp[ch] * env;
        }
    }
    for ch in 0..2 {
        let energy: f32 = out.iter().map(|f| f[ch] * f[ch]).sum();
        let scale = if energy > 0.0 { 0.7 / energy.sqrt() } else { 0.0 };
        for f in &mut out {
            f[ch] *= scale;
        }
    }
    out
}

/// xorshift64*, uniform in [-1, 1). Deterministic so the same settings always
/// give the same file.
fn noise(state: &mut u64) -> f32 {
    let mut x = *state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *state = x;
    let v = x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 40; // 24 bits
    v as f32 / (1u64 << 23) as f32 - 1.0
}

/// 32-bit float stereo WAV bytes.
pub fn wav_bytes(frames: &[[f32; 2]], rate: u32) -> Vec<u8> {
    let data_len = (frames.len() * 8) as u32;
    let mut b = Vec::with_capacity(44 + data_len as usize);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data_len).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&3u16.to_le_bytes()); // IEEE float
    b.extend_from_slice(&2u16.to_le_bytes()); // channels
    b.extend_from_slice(&rate.to_le_bytes());
    b.extend_from_slice(&(rate * 8).to_le_bytes()); // byte rate
    b.extend_from_slice(&8u16.to_le_bytes()); // block align
    b.extend_from_slice(&32u16.to_le_bytes()); // bits
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    for f in frames {
        b.extend_from_slice(&f[0].to_le_bytes());
        b.extend_from_slice(&f[1].to_le_bytes());
    }
    b
}

pub fn cache_dir() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(std::env::temp_dir)
        .join("linuxmeeter")
}

/// Write (or reuse) the impulse response for these settings; returns its path.
pub fn write_reverb_ir(decay_ms: f32, damping: f32) -> io::Result<PathBuf> {
    let dir = cache_dir();
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!(
        "reverb-{}ms-d{}.wav",
        decay_ms.round() as u32,
        (damping.clamp(0.0, 1.0) * 100.0).round() as u32
    ));
    if !path.exists() {
        let tmp = path.with_extension("wav.tmp");
        fs::write(&tmp, wav_bytes(&reverb_ir(decay_ms, damping), IR_RATE))?;
        fs::rename(&tmp, &path)?;
    }
    // Only the current setting is kept; the convolver reads the file once at
    // load, so the module being replaced no longer needs its old one.
    if let Ok(entries) = fs::read_dir(&dir) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with("reverb-") && name.ends_with(".wav") && e.path() != path {
                let _ = fs::remove_file(e.path());
            }
        }
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(frames: &[[f32; 2]], ch: usize) -> f32 {
        (frames.iter().map(|f| f[ch] * f[ch]).sum::<f32>() / frames.len() as f32).sqrt()
    }

    #[test]
    fn length_follows_the_decay_time() {
        assert_eq!(reverb_ir(1000.0, 0.5).len(), 48000);
        assert_eq!(reverb_ir(2500.0, 0.5).len(), 120000);
    }

    /// The tail must actually decay: the last tenth is ~50 dB below the first.
    #[test]
    fn the_tail_decays() {
        let ir = reverb_ir(1500.0, 0.0);
        let tenth = ir.len() / 10;
        let early = rms(&ir[..tenth], 0);
        let late = rms(&ir[ir.len() - tenth..], 0);
        assert!(20.0 * (late / early).log10() < -40.0, "early {early}, late {late}");
    }

    /// Unit energy (times 0.7) per channel, whatever the decay: a longer room
    /// is longer, not louder.
    #[test]
    fn loudness_does_not_depend_on_decay() {
        for d in [300.0, 1800.0, 6000.0] {
            let ir = reverb_ir(d, 0.5);
            for ch in 0..2 {
                let e: f32 = ir.iter().map(|f| f[ch] * f[ch]).sum();
                assert!((e.sqrt() - 0.7).abs() < 1e-3, "decay {d} ch {ch}: energy {e}");
            }
        }
    }

    #[test]
    fn channels_are_decorrelated() {
        let ir = reverb_ir(1000.0, 0.3);
        let dot: f32 = ir.iter().map(|f| f[0] * f[1]).sum();
        assert!(dot.abs() < 0.05, "left/right correlation {dot}");
    }

    /// Damping darkens the tail: the late part has less sample-to-sample
    /// change (high-frequency content) than undamped.
    #[test]
    fn damping_darkens_the_tail() {
        let hf = |ir: &[[f32; 2]]| {
            let late = &ir[ir.len() / 2..];
            let d: f32 = late.windows(2).map(|w| (w[1][0] - w[0][0]).abs()).sum();
            let a: f32 = late.iter().map(|f| f[0].abs()).sum();
            d / a
        };
        assert!(hf(&reverb_ir(2000.0, 1.0)) < hf(&reverb_ir(2000.0, 0.0)) * 0.6);
    }

    #[test]
    fn generation_is_deterministic() {
        assert_eq!(reverb_ir(800.0, 0.4), reverb_ir(800.0, 0.4));
    }

    /// One file per setting, reused, and older settings' files cleaned up so
    /// sweeping the decay knob doesn't fill the cache.
    #[test]
    fn writing_reuses_and_cleans_up() {
        let dir = std::env::temp_dir().join(format!("lm-fx-test-{}", std::process::id()));
        // This is the only test that touches XDG_CACHE_HOME.
        std::env::set_var("XDG_CACHE_HOME", &dir);
        let a = write_reverb_ir(400.0, 0.2).unwrap();
        assert!(a.exists());
        let a_again = write_reverb_ir(400.0, 0.2).unwrap();
        assert_eq!(a, a_again);
        let b = write_reverb_ir(900.0, 0.2).unwrap();
        assert!(b.exists() && !a.exists(), "the previous setting's file is removed");
        assert_eq!(fs::read_dir(cache_dir()).unwrap().count(), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn wav_header_is_float_stereo() {
        let b = wav_bytes(&[[0.5, -0.5], [0.25, 0.0]], 48000);
        assert_eq!(&b[0..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(b[4..8].try_into().unwrap()) as usize, b.len() - 8);
        assert_eq!(&b[8..16], b"WAVEfmt ");
        assert_eq!(u16::from_le_bytes([b[20], b[21]]), 3, "IEEE float");
        assert_eq!(u16::from_le_bytes([b[22], b[23]]), 2, "stereo");
        assert_eq!(u32::from_le_bytes(b[24..28].try_into().unwrap()), 48000);
        assert_eq!(&b[36..40], b"data");
        assert_eq!(u32::from_le_bytes(b[40..44].try_into().unwrap()), 16);
        assert_eq!(f32::from_le_bytes(b[44..48].try_into().unwrap()), 0.5);
        assert_eq!(f32::from_le_bytes(b[48..52].try_into().unwrap()), -0.5);
    }
}
