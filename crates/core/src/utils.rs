//! Small formatting and hashing helpers shared by the app (ported from
//! `utils.py`).

use chrono::{SecondsFormat, Utc};
use std::path::Path;

/// Current UTC time as an ISO-8601 string with second precision
/// (e.g. `2026-08-10T12:34:56Z`).
pub fn utc_now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// `H:MM:SS` when >= 1 hour, else `MM:SS`.
pub fn format_duration(seconds: impl Into<f64>) -> String {
    let value = seconds.into();
    let value = if value.is_finite() { value.max(0.0) } else { 0.0 };
    let total = value as u64;
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let secs = total % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{secs:02}")
    } else {
        format!("{minutes:02}:{secs:02}")
    }
}

/// Human-readable byte size (`512 B`, `12.3 MB`, ...).
pub fn format_bytes(value: impl Into<f64>) -> String {
    let mut size = value.into();
    if !size.is_finite() || size < 0.0 {
        size = 0.0;
    }
    let units = ["B", "KB", "MB", "GB", "TB"];
    for unit in units.iter() {
        if size < 1024.0 || unit == &"TB" {
            return if unit == &"B" {
                format!("{size:.0} {unit}")
            } else {
                format!("{size:.1} {unit}")
            };
        }
        size /= 1024.0;
    }
    "0 B".into()
}

/// Short stable key for media caches: `sha256(resolved|size|mtime)[..24]`.
pub fn media_cache_key(path: &Path, size: u64, mtime: f64) -> String {
    let resolved = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let payload = format!("{}|{}|{mtime:.6}", resolved.display(), size);
    let digest = digest_sha256(payload.as_bytes());
    digest[..24].to_string()
}

fn digest_sha256(bytes: &[u8]) -> String {
    // Small local SHA-256 so the core crate stays dependency-light; the
    // output must match hashlib.sha256(...).hexdigest().
    fn to_hex(bytes: &[u8]) -> String {
        let mut out = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            out.push_str(&format!("{b:02x}"));
        }
        out
    }
    to_hex(&sha2::Sha256::digest(bytes))
}

mod sha2 {
    //! Minimal SHA-256 implementation (FIPS 180-4), table-driven.
    pub struct Sha256 {
        state: [u32; 8],
        buffer: [u8; 64],
        buf_len: usize,
        total_len: u64,
    }

    impl Sha256 {
        pub fn new() -> Self {
            Self {
                state: [
                    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c,
                    0x1f83d9ab, 0x5be0cd19,
                ],
                buffer: [0; 64],
                buf_len: 0,
                total_len: 0,
            }
        }

        pub fn update(&mut self, mut data: &[u8]) {
            self.total_len = self.total_len.wrapping_add(data.len() as u64);
            if self.buf_len > 0 {
                let need = 64 - self.buf_len;
                let take = need.min(data.len());
                self.buffer[self.buf_len..self.buf_len + take].copy_from_slice(&data[..take]);
                self.buf_len += take;
                data = &data[take..];
                if self.buf_len == 64 {
                    let block = self.buffer;
                    compress(&mut self.state, &block);
                    self.buf_len = 0;
                }
            }
            while data.len() >= 64 {
                let mut block = [0u8; 64];
                block.copy_from_slice(&data[..64]);
                compress(&mut self.state, &block);
                data = &data[64..];
            }
            if !data.is_empty() {
                self.buffer[..data.len()].copy_from_slice(data);
                self.buf_len = data.len();
            }
        }

        pub fn finalize(mut self) -> [u8; 32] {
            let bit_len = self.total_len.wrapping_mul(8);
            let mut tail = Vec::with_capacity(72);
            tail.push(0x80);
            let zeroes = (64 - ((self.buf_len + 9) % 64)) % 64;
            tail.extend(std::iter::repeat_n(0u8, zeroes));
            tail.extend_from_slice(&bit_len.to_be_bytes());
            self.update(&tail);
            debug_assert_eq!(self.buf_len, 0);
            let mut out = [0u8; 32];
            for (i, word) in self.state.iter().enumerate() {
                out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
            }
            out
        }

        pub fn digest(bytes: &[u8]) -> [u8; 32] {
            let mut hasher = Self::new();
            hasher.update(bytes);
            hasher.finalize()
        }
    }

    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    fn compress(state: &mut [u32; 8], block: &[u8; 64]) {
        let mut w = [0u32; 64];
        for (i, chunk) in block.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }
}

/// Filesystem-safe stem derived from a media file name.
pub fn safe_stem(name: &str, max_length: usize) -> String {
    let path = Path::new(name);
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let cleaned: String = stem
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let cleaned = cleaned.trim_matches(|c| c == '.' || c == '_' || c == '-').to_string();
    let cleaned = if cleaned.is_empty() { "clip".to_string() } else { cleaned };
    cleaned.chars().take(max_length).collect()
}

pub fn clamp(value: f64, minimum: f64, maximum: f64) -> f64 {
    value.max(minimum).min(maximum)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_known_vector() {
        // "abc" -> ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
        let digest = digest_sha256(b"abc");
        assert_eq!(
            digest,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn sha256_matches_empty() {
        assert_eq!(
            digest_sha256(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn duration_format() {
        assert_eq!(format_duration(0), "00:00");
        assert_eq!(format_duration(59), "00:59");
        assert_eq!(format_duration(60), "01:00");
        assert_eq!(format_duration(3599), "59:59");
        assert_eq!(format_duration(3600), "1:00:00");
        assert_eq!(format_duration(3661.5), "1:01:01");
    }

    #[test]
    fn bytes_format() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(12_345_678), "11.8 MB");
        assert_eq!(format_bytes(5_000_000_000.0), "4.7 GB");
    }

    #[test]
    fn safe_stem_rules() {
        assert_eq!(safe_stem("My Video 01.mp4", 72), "My_Video_01");
        assert_eq!(safe_stem("..hidden..mkv", 72), "hidden");
        assert_eq!(safe_stem("clip", 72), "clip");
        assert_eq!(
            safe_stem(&("a".repeat(100) + ".mp4"), 72).len(),
            72
        );
    }

    #[test]
    fn cache_key_is_stable() {
        let p = std::path::Path::new("/tmp/nonexistent/x.mp4");
        // Cross-implementation vector: must match hashlib.sha256 of
        // `{resolved}|{size}|{mtime:.6f}` truncated to 24 chars.
        // The file must exist so canonicalize() resolves /tmp -> /private/tmp
        // exactly like Python's Path.resolve().
        let dir = tempfile::tempdir().unwrap();
        let probe = dir.path().join("clip.mp4");
        std::fs::write(&probe, b"x").unwrap();
        let resolved = probe.canonicalize().unwrap();
        let key = media_cache_key(&probe, 12345, 1700000000.123456);
        let expected_payload = format!("{}|{}|{:.6}", resolved.display(), 12345, 1700000000.123456);
        let digest = digest_sha256(expected_payload.as_bytes());
        assert_eq!(key, &digest[..24]);

        let a = media_cache_key(p, 10, 1.5);
        let b = media_cache_key(p, 10, 1.5);
        assert_eq!(a, b);
        assert_eq!(a.len(), 24);
        let c = media_cache_key(p, 11, 1.5);
        assert_ne!(a, c);
    }
}
