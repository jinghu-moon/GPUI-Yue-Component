# ImagePreview fixtures

User-provided screenshots, relocated unchanged from the repository root on 2026-11-02.
These are Showcase/test assets, not assets bundled by the component library.

| File | Pixels | Origin |
| --- | --- | --- |
| desktop.png | 3840 × 2088 | Original `图片.png`, SHA256 `3c12e5af5f7c94d3d323917e59c98404f1e9ad5d464429d054a9e4931c77fc59` |
| long-screenshot.png | 3840 × 4998 | Original `长截图.png`, SHA256 `2237cf5acaa8a451c33d2707989dbf9bda8f4f646c5ce2fb42d09a13bb104e69` |
| long-preview.jpg | 1024 × 1333 | Lanczos preview proxy generated from `long-screenshot.png` for fast first paint |
| desktop-q70.jpg | 3840 × 2088 | Actual JPEG encoding of desktop.png, Pillow quality=70, optimize=True |
| desktop-half-q70.jpg | 1920 × 1044 | desktop.png resized using Lanczos, then JPEG quality=70, optimize=True |
| transparent.png | 640 × 400 | Generated RGBA fixture: translucent rounded rectangle and ellipse |

The JPEGs demonstrate real encoding and resizing, not a blur filter. The half-size
sample compares the combined resize/encoding result, not isolated compression loss.
The long screenshot contains small text for FitWidth, panning and 100% checks.

The Showcase's “传入多张图片” switch reads supported images from
`C:\Users\seeyuer\桌面\新建文件夹` in filename order. Files are discovered and
decoded on four background thumbnail workers, prioritized around the current
selection. A 256px thumbnail doubles as an immediate display proxy; a separate
worker prepares the selected image and its neighbors at up to 2048px / 12MiB.
Six prepared main images are retained in an LRU cache. Small decoded thumbnails
are persisted under `%LOCALAPPDATA%/YueComponent/image-preview-thumbnails-v2`,
keyed by path, file length, modification time and format version. Corrupt entries
are regenerated. The directory is a local demo input; its images are not copied
into the repository and the component library owns no file loader or cache.

Set `YUE_IMAGE_PREVIEW_TIMING=1` before startup to log queue, I/O, decode,
resampling, cache, publication and first paint timings. See the
[timing guide and measurements](../../../../docs/performance/2026-11-07-image-preview-timing.md).
