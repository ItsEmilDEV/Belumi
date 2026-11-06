use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

pub const SIZE: usize = 8;
const BYTES: usize = SIZE * SIZE * 4;
const REFRESH_AFTER: Duration = Duration::from_secs(24 * 3600);

fn path(dir: &Path, uuid: &str) -> PathBuf {
    let safe: String = uuid.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    dir.join(format!("{safe}.rgba"))
}

fn skin_path(dir: &Path, uuid: &str) -> PathBuf {
    path(dir, uuid).with_extension("skin")
}

pub const SKIN_BYTES: usize = 64 * 64 * 4;

pub fn skin_cached(dir: &Path, uuid: &str) -> Option<Vec<u8>> {
    let bytes = std::fs::read(skin_path(dir, uuid)).ok()?;
    (bytes.len() == SKIN_BYTES).then_some(bytes)
}

pub fn cached(dir: &Path, uuid: &str) -> Option<Vec<u8>> {
    let bytes = std::fs::read(path(dir, uuid)).ok()?;
    (bytes.len() == BYTES).then_some(bytes)
}

pub fn is_stale(dir: &Path, uuid: &str) -> bool {
    if !skin_path(dir, uuid).exists() {
        return true;
    }
    std::fs::metadata(path(dir, uuid))
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .is_none_or(|age| age > REFRESH_AFTER)
}

pub fn fetch_and_store(dir: &Path, uuid: &str) -> Option<Vec<u8>> {
    let (head, skin) = fetch(uuid)?;
    let _ = std::fs::create_dir_all(dir);
    let _ = std::fs::write(path(dir, uuid), &head);
    let _ = std::fs::write(skin_path(dir, uuid), &skin);
    Some(head)
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(15))
        .build()
}

fn fetch(uuid: &str) -> Option<(Vec<u8>, Vec<u8>)> {
    let id: String = uuid.chars().filter(|c| *c != '-').collect();
    let profile: Value = agent()
        .get(&format!(
            "https://sessionserver.mojang.com/session/minecraft/profile/{id}"
        ))
        .call()
        .ok()?
        .into_json()
        .ok()?;
    let value = profile["properties"]
        .as_array()?
        .iter()
        .find(|p| p["name"] == "textures")?["value"]
        .as_str()?;
    let textures: Value = serde_json::from_slice(&base64_decode(value)?).ok()?;
    let url = textures["textures"]["SKIN"]["url"].as_str()?;
    let mut png = Vec::new();
    std::io::Read::read_to_end(
        &mut std::io::Read::take(agent().get(url).call().ok()?.into_reader(), 1 << 20),
        &mut png,
    )
    .ok()?;
    let icon = eframe::icon_data::from_png_bytes(&png).ok()?;
    let (w, h) = (icon.width as usize, icon.height as usize);
    Some((head_of(&icon.rgba, w, h)?, normalize_skin(&icon.rgba, w, h)?))
}

/// Any skin as the classic 64 x 64 layout: HD skins sampled down, old 64 x 32 skins get a mirrored left arm and leg.
pub fn normalize_skin(rgba: &[u8], width: usize, height: usize) -> Option<Vec<u8>> {
    if width < 64 || width % 64 != 0 || (height != width && height != width / 2) || rgba.len() < width * height * 4 {
        return None;
    }
    let scale = width / 64;
    let rows = height / scale;
    let mut out = vec![0u8; SKIN_BYTES];
    for y in 0..rows {
        for x in 0..64 {
            let at = ((y * scale) * width + x * scale) * 4;
            out[(y * 64 + x) * 4..][..4].copy_from_slice(&rgba[at..at + 4]);
        }
    }
    if rows == 32 {
        let mirror = |out: &mut Vec<u8>, from: (usize, usize), to: (usize, usize), w: usize, h: usize| {
            for y in 0..h {
                for x in 0..w {
                    let s = ((from.1 + y) * 64 + from.0 + (w - 1 - x)) * 4;
                    let d = ((to.1 + y) * 64 + to.0 + x) * 4;
                    let px: [u8; 4] = out[s..s + 4].try_into().unwrap_or([0; 4]);
                    out[d..d + 4].copy_from_slice(&px);
                }
            }
        };
        for (src, dst) in [((0, 16), (16, 48)), ((40, 16), (32, 48))] {
            let faces: [((usize, usize), (usize, usize), usize, usize); 6] = [
                ((4, 0), (4, 0), 4, 4),
                ((8, 0), (8, 0), 4, 4),
                ((0, 4), (8, 4), 4, 12),
                ((4, 4), (4, 4), 4, 12),
                ((8, 4), (0, 4), 4, 12),
                ((12, 4), (12, 4), 4, 12),
            ];
            for (f, t, w, h) in faces {
                mirror(&mut out, (src.0 + f.0, src.1 + f.1), (dst.0 + t.0, dst.1 + t.1), w, h);
            }
        }
    }
    Some(out)
}

pub fn head_of(rgba: &[u8], width: usize, height: usize) -> Option<Vec<u8>> {
    if width < 64 || width % 64 != 0 || height < width / 2 || rgba.len() < width * height * 4 {
        return None;
    }
    let scale = width / 64;
    let pixel = |x: usize, y: usize| {
        let at = ((y * scale) * width + x * scale) * 4;
        [rgba[at], rgba[at + 1], rgba[at + 2], rgba[at + 3]]
    };
    let mut out = Vec::with_capacity(BYTES);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let face = pixel(8 + x, 8 + y);
            let hat = pixel(40 + x, 8 + y);
            let a = f32::from(hat[3]) / 255.0;
            for c in 0..3 {
                out.push((f32::from(hat[c]) * a + f32::from(face[c]) * (1.0 - a)) as u8);
            }
            out.push(255);
        }
    }
    Some(out)
}

fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let (mut bits, mut count) = (0u32, 0);
    for c in text.bytes().filter(|c| *c != b'=') {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => return None,
        };
        bits = (bits << 6) | u32::from(v);
        count += 6;
        if count >= 8 {
            count -= 8;
            out.push((bits >> count) as u8);
            bits &= (1 << count) - 1;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_skins_get_a_left_arm_and_leg() {
        let mut old = vec![0u8; 64 * 32 * 4];
        old[(20 * 64 + 5) * 4..][..4].copy_from_slice(&[9, 9, 9, 255]);
        let skin = normalize_skin(&old, 64, 32).unwrap();
        assert_eq!(skin.len(), SKIN_BYTES);
        assert!(skin.chunks(4).any(|p| p == [9, 9, 9, 255]));
        assert!(normalize_skin(&old, 64, 48).is_none());
    }

    #[test]
    fn base64_reads_texture_values() {
        assert_eq!(base64_decode("aGVsbG8=").unwrap(), b"hello");
        assert!(base64_decode("a*b").is_none());
    }

    #[test]
    fn the_head_is_the_face_with_the_hat_over_it() {
        let (w, h) = (64, 64);
        let mut img = vec![0u8; w * h * 4];
        let set = |img: &mut Vec<u8>, x: usize, y: usize, p: [u8; 4]| {
            img[(y * w + x) * 4..][..4].copy_from_slice(&p);
        };
        for y in 0..8 {
            for x in 0..8 {
                set(&mut img, 8 + x, 8 + y, [10, 20, 30, 255]);
            }
        }
        set(&mut img, 40, 8, [200, 0, 0, 255]);
        let head = head_of(&img, w, h).unwrap();
        assert_eq!(head.len(), 256);
        assert_eq!(&head[0..4], &[200, 0, 0, 255]);
        assert_eq!(&head[4..8], &[10, 20, 30, 255]);
        assert!(head_of(&img[..100], w, h).is_none());
    }

    #[test]
    fn the_cache_round_trips_and_ages() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cached(dir.path(), "ab-cd").is_none());
        assert!(is_stale(dir.path(), "ab-cd"));
        std::fs::write(path(dir.path(), "ab-cd"), vec![1u8; BYTES]).unwrap();
        assert!(is_stale(dir.path(), "ab-cd"), "a head without its skin file is refetched");
        std::fs::write(skin_path(dir.path(), "ab-cd"), vec![1u8; SKIN_BYTES]).unwrap();
        assert_eq!(cached(dir.path(), "ab-cd").unwrap().len(), BYTES);
        assert_eq!(skin_cached(dir.path(), "ab-cd").unwrap().len(), SKIN_BYTES);
        assert!(!is_stale(dir.path(), "ab-cd"));
    }

    #[test]
    #[ignore = "asks Mojang's servers"]
    fn a_real_profile_gives_a_head() {
        let (head, skin) = fetch("069a79f444e94726a5befca90e38aaf5").unwrap();
        assert_eq!(head.len(), 256);
        assert_eq!(skin.len(), SKIN_BYTES);
    }
}
