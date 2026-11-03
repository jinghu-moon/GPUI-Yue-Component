//! Pure color math and ICC preparation. No display, file I/O or retained state.
//! Working RGB uses linear Rec.709 primaries; absolute scRGB units are 80 nit.
use moxcms::{ColorProfile, ToneReprCurve};

pub(crate) fn linear_srgb_profile() -> ColorProfile {
    let mut profile = ColorProfile::new_srgb();
    let curve = Some(ToneReprCurve::Parametric(vec![1.0]));
    profile.red_trc = curve.clone();
    profile.green_trc = curve.clone();
    profile.blue_trc = curve;
    profile.cicp = None;
    profile
}

pub(crate) fn srgb_to_linear(value: f64) -> f64 {
    let magnitude = value.abs();
    let result = if magnitude <= 0.04045 {
        magnitude / 12.92
    } else {
        ((magnitude + 0.055) / 1.055).powf(2.4)
    };
    result.copysign(value)
}

pub(crate) fn linear_to_srgb(value: f64) -> f64 {
    let magnitude = value.abs();
    let result = if magnitude <= 0.0031308 {
        magnitude * 12.92
    } else {
        1.055 * magnitude.powf(1.0 / 2.4) - 0.055
    };
    result.copysign(value)
}

// BT.2100 / ST 2084. Input is full-range normalized code, output absolute nit.
pub(crate) fn pq_to_nits(code: f64) -> f64 {
    let p = code.powf(32.0 / 2523.0);
    let numerator = (p - 3424.0 / 4096.0).max(0.0);
    let denominator = 2413.0 / 128.0 - 2392.0 / 128.0 * p;
    10000.0 * (numerator / denominator).powf(16384.0 / 2610.0)
}

pub(crate) fn nits_to_pq(nits: f64) -> f64 {
    let p = (nits / 10000.0).powf(2610.0 / 16384.0);
    ((3424.0 / 4096.0 + 2413.0 / 128.0 * p) / (1.0 + 2392.0 / 128.0 * p)).powf(2523.0 / 32.0)
}

// D65 to D65 matrix, derived from the BT.2020 and BT.709 chromaticities.
// Values outside [0, 1] intentionally preserve extended color gamut.
pub(crate) fn rec2020_to_rec709(rgb: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = rgb;
    [
        1.6604910021084345 * r - 0.5876411387885495 * g - 0.0728498633198848 * b,
        -0.1245504745215907 * r + 1.1328998971259598 * g - 0.0083494226043692 * b,
        -0.0181507633549053 * r - 0.1005788980080074 * g + 1.1187296613629127 * b,
    ]
}

#[cfg(test)]
pub(crate) fn rec709_to_rec2020(rgb: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = rgb;
    [
        0.627403895934699 * r + 0.329283038377883 * g + 0.043313065687418 * b,
        0.069097289358232 * r + 0.919540395075459 * g + 0.011362315566309 * b,
        0.016391438875150 * r + 0.088013307877226 * g + 0.895595253247624 * b,
    ]
}

/// BT.2390 EETF with zero source/target black. Hermite roll-off in normalized
/// PQ space; no exposure change below the knee. This is a display derivative,
/// never a mutation of the absolute source. Inputs are validated by the caller.
pub(crate) fn map_luminance(nits: f64, source_peak: f64, target_peak: f64) -> f64 {
    if source_peak <= target_peak || nits <= 0.0 {
        return nits.max(0.0);
    }
    let black = nits_to_pq(0.0);
    let source_range = nits_to_pq(source_peak.min(10000.0)) - black;
    let maximum = (nits_to_pq(target_peak.min(10000.0)) - black) / source_range;
    let knee = (1.5 * maximum - 0.5).clamp(0.0, 1.0);
    let input = ((nits_to_pq(nits.min(source_peak)) - black) / source_range).clamp(0.0, 1.0);
    let output = if input <= knee || knee >= 1.0 {
        input
    } else {
        let t = (input - knee) / (1.0 - knee);
        let t2 = t * t;
        let t3 = t2 * t;
        (2.0 * t3 - 3.0 * t2 + 1.0) * knee
            + (t3 - 2.0 * t2 + t) * (1.0 - knee)
            + (-2.0 * t3 + 3.0 * t2) * maximum
    };
    pq_to_nits(output * source_range + black)
}

/// Absolute scRGB to relative linear SDR. The target's logical white/peak is
/// fixed at 100 nit. Compress chroma toward the same luminance only when RGB
/// leaves the target cube; per-channel clipping would shift hues.
pub(crate) fn scrgb_to_sdr(rgb: [f64; 3], source_peak: f64) -> [f64; 3] {
    let luminance = 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2];
    if luminance <= 0.0 {
        return [0.0; 3];
    }
    // PQ round-trip arithmetic may put the mapped peak a few ulps above white.
    // Bound the target luminance before computing chroma intersections.
    let mapped =
        (map_luminance(luminance * 80.0, source_peak.max(100.0), 100.0) / 100.0).clamp(0.0, 1.0);
    let scaled = rgb.map(|v| v / luminance * mapped);
    let mut chroma = 1_f64;
    for value in scaled {
        if value < 0.0 {
            chroma = chroma.min(mapped / (mapped - value));
        } else if value > 1.0 {
            chroma = chroma.min((1.0 - mapped) / (value - mapped));
        }
    }
    scaled.map(|v| (mapped + (v - mapped) * chroma.clamp(0.0, 1.0)).clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    include!("../../tests/fixtures/hdr/sdr-vectors.rs");

    #[test]
    fn cpu_mapping_matches_independent_quantized_oracle() {
        for &(rgb, peak, expected) in SDR_MAPPING_VECTORS {
            for (actual, expected) in scrgb_to_sdr(rgb, peak).into_iter().zip(expected) {
                close(actual, expected, 1e-9);
            }
        }
    }

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "{actual} != {expected}"
        );
    }

    #[test]
    fn pq_reference_codes_and_absolute_scrgb_white() {
        for (nits, code) in [
            (100.0, 0.508078421517399),
            (203.0, 0.5806888810416109),
            (1000.0, 0.751827096247041),
            (10000.0, 1.0),
        ] {
            close(nits_to_pq(nits), code, 1e-12);
            close(pq_to_nits(code), nits, 1e-6);
        }
        close(pq_to_nits(0.0), 0.0, 1e-12);
        close(pq_to_nits(nits_to_pq(203.0)) / 80.0, 2.5375, 1e-9);
    }

    #[test]
    fn srgb_uses_piecewise_transfer_and_preserves_extended_signs() {
        close(srgb_to_linear(0.04045), 0.00313080495356, 1e-12);
        close(linear_to_srgb(0.0031308), 0.040449936, 1e-12);
        for v in [-1.3, -0.04, 0.0, 0.2, 0.5, 1.0, 2.5] {
            close(linear_to_srgb(srgb_to_linear(v)), v, 1e-7);
        }
    }

    #[test]
    fn primary_conversion_keeps_negative_and_superwhite_components() {
        let primary = rec2020_to_rec709([1.0, 0.0, 0.0]);
        assert!(primary[0] > 1.0 && primary[1] < 0.0 && primary[2] < 0.0);
        for rgb in [[1.0; 3], [12.5; 3], [0.2, -0.1, 2.0]] {
            for (actual, expected) in rec709_to_rec2020(rec2020_to_rec709(rgb))
                .into_iter()
                .zip(rgb)
            {
                close(actual, expected, 1e-12);
            }
        }
    }

    #[test]
    fn eetf_is_monotonic_maps_the_peak_and_does_not_compress_in_range_content() {
        for (source, target) in [(1000.0, 100.0), (10000.0, 100.0), (1000.0, 600.0)] {
            close(map_luminance(source, source, target), target, 1e-6);
            let mut previous = -1.0;
            for i in 0..=1000 {
                let value = map_luminance(source * f64::from(i) / 1000.0, source, target);
                assert!(value >= previous - 1e-8 && value <= target + 1e-6);
                previous = value;
            }
        }
        close(map_luminance(203.0, 400.0, 1000.0), 203.0, 1e-9);
    }

    #[test]
    fn sdr_mapping_retains_neutrality_and_gamut_compresses_instead_of_clipping() {
        for gray in [0.0, 1.0, 2.5375, 12.5, 125.0] {
            let output = scrgb_to_sdr([gray; 3], 10000.0);
            close(output[0], output[1], 1e-12);
            close(output[1], output[2], 1e-12);
            assert!(output.iter().all(|v| (0.0..=1.0).contains(v)));
        }
        let output = scrgb_to_sdr(rec2020_to_rec709([3.0, 0.1, 0.1]), 1000.0);
        assert!(
            output
                .iter()
                .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
        );
    }

    #[test]
    fn icc_to_linear_f32_keeps_precision_and_alpha() {
        let source = ColorProfile::new_from_slice(include_bytes!(
            "../../tests/fixtures/hdr/srgb-matrix.icc"
        ))
        .unwrap();
        let transform = source
            .create_transform_f32(
                moxcms::Layout::Rgba,
                &linear_srgb_profile(),
                moxcms::Layout::Rgba,
                moxcms::TransformOptions::default(),
            )
            .unwrap();
        let input = [0.25, 0.5, 0.75, 0.37];
        let mut output = [0.0; 4];
        transform.transform(&input, &mut output).unwrap();
        for ix in 0..3 {
            close(
                f64::from(output[ix]),
                srgb_to_linear(f64::from(input[ix])),
                0.0002,
            );
        }
        close(f64::from(output[3]), f64::from(input[3]), 1e-7);
    }
}
