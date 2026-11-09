//! Small decoded thumbnails, keyed by source identity/size/mtime and cache version.
//! No full images are persisted; corrupt or obsolete entries are ordinary misses.
use super::*;
use gpui_kit::{RenderImage, RenderImageFormat};
use std::{
    hash::{Hash, Hasher},
    time::UNIX_EPOCH,
};

const MAGIC: &[u8; 8] = b"YIPTHM02";
const HEADER: usize = 32;
const MAX_BYTES: u64 = (HEADER + 256 * 256 * 8) as u64;

pub(super) fn key(path: &Path) -> std::io::Result<u64> {
    let metadata = path.metadata()?;
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    MAGIC.hash(&mut hash);
    path.hash(&mut hash);
    metadata.len().hash(&mut hash);
    metadata
        .modified()?
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .hash(&mut hash);
    Ok(hash.finish())
}
fn root() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("YueComponent")
        .join("image-preview-thumbnails-v2")
}
fn file(key: u64) -> PathBuf {
    root().join(format!("{key:016x}.thumb"))
}

pub(super) fn read(key: u64, name: SharedString) -> Option<ImagePreviewImage> {
    unpack(&file(key), name)
}
fn unpack(path: &Path, name: SharedString) -> Option<ImagePreviewImage> {
    let length = path.metadata().ok()?.len();
    if !(HEADER as u64..=MAX_BYTES).contains(&length) {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    if bytes.get(..8)? != MAGIC {
        return None;
    }
    let word = |offset: usize| {
        Some(u32::from_le_bytes(
            bytes.get(offset..offset + 4)?.try_into().ok()?,
        ))
    };
    let original = [word(8)?, word(12)?];
    let [width, height] = [word(16)?, word(20)?];
    if width == 0 || height == 0 || width > 256 || height > 256 {
        return None;
    }
    let raw = bytes.get(HEADER..)?;
    let kind = *bytes.get(24)?;
    let image = if kind == 0 {
        ImagePreviewPixels::from_bgra8(width, height, raw)
            .ok()?
            .into_preview(&ImagePreviewLimits::default(), name)
            .ok()?
    } else {
        let format = match kind {
            1 => RenderImageFormat::Rgba16FloatLinearSrgb,
            2 => RenderImageFormat::Rgba16FloatScRgb,
            _ => return None,
        };
        let peak = f32::from_bits(word(28)?);
        let peak = if peak.is_nan() { None } else { Some(peak) };
        let resource = RenderImage::from_rgba16f_with_source_peak(
            gpui_kit::size((width as i32).into(), (height as i32).into()),
            raw.to_vec(),
            format,
            peak,
        )
        .ok()?;
        ImagePreviewImage::new(Arc::new(resource), name).ok()?
    };
    image.with_source_size(original).ok()
}

pub(super) fn write(key: u64, image: &ImagePreviewImage) {
    let destination = file(key);
    if std::fs::create_dir_all(root()).is_err() {
        return;
    }
    let bytes = pack(image);
    let temporary = destination.with_extension("tmp");
    if std::fs::write(&temporary, bytes).is_ok() {
        let _ = std::fs::rename(&temporary, destination);
    }
}
fn pack(image: &ImagePreviewImage) -> Vec<u8> {
    let mut bytes = MAGIC.to_vec();
    for dimension in image.pixel_size().into_iter().chain(image.texture_size()) {
        bytes.extend_from_slice(&dimension.to_le_bytes());
    }
    bytes.push(match image.image().format() {
        RenderImageFormat::Bgra8Srgb => 0,
        RenderImageFormat::Rgba16FloatLinearSrgb => 1,
        RenderImageFormat::Rgba16FloatScRgb => 2,
    });
    bytes.extend_from_slice(&[0; 3]);
    bytes.extend_from_slice(
        &image
            .image()
            .peak_nits()
            .unwrap_or(f32::NAN)
            .to_bits()
            .to_le_bytes(),
    );
    bytes.extend_from_slice(image.image().as_bytes(0).unwrap_or_default());
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_round_trip_keeps_pixels_dimensions_and_rejects_corruption() {
        let dir = std::env::temp_dir().join(format!("yue-thumbnail-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sample.thumb");
        let image =
            ImagePreviewImage::from_rgba8(2, 1, vec![10, 20, 30, 255, 40, 50, 60, 127], "sample")
                .unwrap()
                .with_source_size([200, 100])
                .unwrap();
        std::fs::write(&path, pack(&image)).unwrap();
        let restored = unpack(&path, "restored".into()).unwrap();
        assert_eq!(restored.pixel_size(), [200, 100]);
        assert_eq!(restored.image().as_bytes(0), image.image().as_bytes(0));
        let hdr = ImagePreviewImage::from_encoded(
            include_bytes!("../../../../../../crates/yue-component/tests/fixtures/hdr/pq-gray.png"),
            &ImagePreviewLimits::default(),
            "HDR",
        )
        .unwrap();
        std::fs::write(&path, pack(&hdr)).unwrap();
        let restored = unpack(&path, "restored HDR".into()).unwrap();
        assert_eq!(restored.color_space(), hdr.color_space());
        assert_eq!(restored.image().peak_nits(), hdr.image().peak_nits());
        assert_eq!(restored.image().as_bytes(0), hdr.image().as_bytes(0));
        std::fs::write(&path, b"truncated").unwrap();
        assert!(unpack(&path, "bad".into()).is_none());
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_dir(dir);
    }
    #[test]
    fn changed_sources_do_not_reuse_old_thumbnail_keys() {
        let path = std::env::temp_dir().join(format!("yue-thumbnail-key-{}", std::process::id()));
        std::fs::write(&path, b"first").unwrap();
        let first = key(&path).unwrap();
        std::fs::write(&path, b"changed source").unwrap();
        assert_ne!(key(&path).unwrap(), first);
        let _ = std::fs::remove_file(path);
    }
}
