#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Listener {
    pub pos: [f64; 3],
    pub yaw: f32,
}

pub fn distance_gain(max_distance: f32, listener: &Listener, source: [f64; 3]) -> f32 {
    if max_distance <= 0.0 {
        return 1.0;
    }
    let d = ((source[0] - listener.pos[0]).powi(2)
        + (source[1] - listener.pos[1]).powi(2)
        + (source[2] - listener.pos[2]).powi(2))
    .sqrt() as f32;
    1.0 - d.min(max_distance) / max_distance
}

pub fn pan(listener: &Listener, source: [f64; 3]) -> f32 {
    let dx = source[0] - listener.pos[0];
    let dz = source[2] - listener.pos[2];
    let len = dx.hypot(dz);
    if len < 1e-6 {
        return 0.0;
    }
    let (sy, cy) = f64::from(listener.yaw).to_radians().sin_cos();
    let right = [-cy, -sy];
    ((dx * right[0] + dz * right[1]) / len) as f32
}

pub fn stereo_gains(listener: &Listener, source: [f64; 3]) -> (f32, f32) {
    let p = (pan(listener, source) * 0.8).clamp(-1.0, 1.0);
    let angle = (p + 1.0) * std::f32::consts::FRAC_PI_4;
    (angle.cos(), angle.sin())
}

#[cfg(test)]
mod tests {
    use super::*;

    const L: Listener = Listener {
        pos: [0.0, 64.0, 0.0],
        yaw: 0.0,
    };

    #[test]
    fn volume_falls_linearly_to_silence_at_the_maximum_distance() {
        assert_eq!(distance_gain(48.0, &L, [0.0, 64.0, 0.0]), 1.0);
        assert!((distance_gain(48.0, &L, [24.0, 64.0, 0.0]) - 0.5).abs() < 1e-6);
        assert_eq!(distance_gain(48.0, &L, [100.0, 64.0, 0.0]), 0.0);
        assert_eq!(distance_gain(0.0, &L, [5.0, 64.0, 5.0]), 1.0);
    }

    #[test]
    fn sources_pan_toward_the_side_they_are_on() {
        let (l, r) = stereo_gains(&L, [-10.0, 64.0, 0.0]);
        assert!(r > l, "facing south, west is on the right");
        let (l, r) = stereo_gains(&L, [10.0, 64.0, 0.0]);
        assert!(l > r);
        let (l, r) = stereo_gains(&L, [0.0, 64.0, 10.0]);
        assert!((l - r).abs() < 1e-3);
        let (l, r) = stereo_gains(&L, [0.0, 64.0, 0.0]);
        assert!((l - r).abs() < 1e-3);
    }

    #[test]
    fn turning_swaps_the_sides() {
        let turned = Listener { yaw: 180.0, ..L };
        let (l, r) = stereo_gains(&turned, [-10.0, 64.0, 0.0]);
        assert!(l > r);
    }

    #[test]
    fn panning_keeps_the_total_power_constant() {
        for x in [-20.0, -5.0, 0.0, 5.0, 20.0] {
            let (l, r) = stereo_gains(&L, [x, 64.0, 3.0]);
            assert!((l * l + r * r - 1.0).abs() < 1e-3);
        }
    }
}
