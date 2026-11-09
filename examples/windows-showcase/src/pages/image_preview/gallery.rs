//! Independent thumbnail and selected-image queues. All disk/CPU work stays off
//! the UI thread, and every completion is scoped to its gallery generation.
mod cache;
use super::timing::{Clock, Profile};
use super::*;
use std::{
    collections::{HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::Arc,
};
use yue_component::ImagePreviewPixels;

const DIRECTORY: &str = r"C:\Users\seeyuer\桌面\新建文件夹";
const SOURCE_CACHE_SIZE: usize = 6;
const THUMBNAIL_WORKERS: usize = 4;

#[derive(Clone)]
struct Entry {
    path: PathBuf,
    name: SharedString,
    key: u64,
}

pub(super) struct GallerySession {
    entries: Arc<Vec<Entry>>,
    pending: Vec<usize>,
    ready: HashSet<usize>,
    attempted: HashSet<usize>,
    recent: VecDeque<usize>,
    source_running: bool,
    profile: Profile,
    queued_at: Clock,
    remaining_thumbnails: usize,
    pending_proxy: Option<(usize, Clock)>,
    pending_source: Option<(usize, Clock)>,
}
impl GallerySession {
    pub(super) fn finish_profile(&self) {
        self.profile.summary("gallery_replaced");
    }
    fn next_thumbnail(&mut self, active: usize) -> Option<usize> {
        let slot = self
            .pending
            .iter()
            .enumerate()
            .min_by_key(|(_, index)| index.abs_diff(active))?
            .0;
        Some(self.pending.remove(slot))
    }
    fn next_source(&mut self, active: usize) -> Option<usize> {
        // Selection always wins over prefetch, even after a job was already started.
        let next = [Some(active), active.checked_add(1), active.checked_sub(1)]
            .into_iter()
            .flatten()
            .find(|index| {
                *index < self.entries.len()
                    && !self.ready.contains(index)
                    && !self.attempted.contains(index)
            })?;
        self.attempted.insert(next);
        Some(next)
    }
    fn touch(&mut self, index: usize) {
        self.recent.retain(|previous| *previous != index);
        self.recent.push_back(index);
    }
    fn report_ready(&mut self, index: usize, full: bool) {
        if self
            .pending_proxy
            .is_some_and(|(selected, _)| selected == index)
        {
            let (_, at) = self.pending_proxy.take().unwrap();
            self.profile
                .record("selection_to_preview_ready", &self.entries[index].name, at);
        }
        if full
            && self
                .pending_source
                .is_some_and(|(selected, _)| selected == index)
        {
            let (_, at) = self.pending_source.take().unwrap();
            self.profile
                .record("selection_to_full_ready", &self.entries[index].name, at);
        }
    }
}

fn limits() -> ImagePreviewLimits {
    ImagePreviewLimits::default()
        .max_pixels(96_000_000)
        .texture_budget(2048, 12 * 1024 * 1024)
}
fn paths() -> Result<Vec<Entry>, String> {
    let mut files = std::fs::read_dir(DIRECTORY)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path.extension().is_some_and(|extension| {
                    matches!(
                        extension.to_string_lossy().to_ascii_lowercase().as_str(),
                        "png" | "jpg" | "jpeg" | "webp" | "bmp"
                    )
                })
        })
        .collect::<Vec<_>>();
    files.sort();
    if files.is_empty() {
        return Err("图片目录中没有支持的图片".into());
    }
    files
        .into_iter()
        .map(|path| {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
                .into();
            let key = cache::key(&path).map_err(|error| error.to_string())?;
            Ok(Entry { path, name, key })
        })
        .collect()
}
fn decode(
    path: &Path,
    quick: bool,
    name: SharedString,
    profile: &Profile,
) -> Result<ImagePreviewImage, String> {
    let bytes = profile
        .run("file_read", &name, || std::fs::read(path))
        .map_err(|error| error.to_string())?;
    let limits = limits();
    let pixels = profile
        .run(
            if quick {
                "thumbnail_decode"
            } else {
                "source_decode"
            },
            &name,
            || ImagePreviewPixels::decode(&bytes, &limits),
        )
        .map_err(|error| error.to_string())?;
    let image = profile
        .run(
            if quick {
                "thumbnail_resize"
            } else {
                "source_resize"
            },
            &name,
            || {
                if quick {
                    pixels.into_quick_preview(
                        &limits.texture_budget(256, 256 * 256 * 8),
                        name.clone(),
                    )
                } else {
                    pixels.into_preview(&limits, name.clone())
                }
            },
        )
        .map_err(|error| error.to_string())?;
    profile.note(format_args!(
        "resource={name:?} quick={quick} encoded_bytes={} source={:?} texture={:?}",
        bytes.len(),
        image.pixel_size(),
        image.texture_size()
    ));
    Ok(image)
}
fn read_cached(entry: &Entry, profile: &Profile) -> Option<ImagePreviewImage> {
    let started = profile.clock();
    let result = cache::read(entry.key, entry.name.clone());
    profile.record(
        if result.is_some() {
            "cache_hit"
        } else {
            "cache_miss"
        },
        &entry.name,
        started,
    );
    result
}
fn thumbnail(entry: &Entry, profile: &Profile) -> Result<ImagePreviewImage, String> {
    if let Some(image) = read_cached(entry, profile) {
        return Ok(image);
    }
    let image = decode(&entry.path, true, entry.name.clone(), profile)?;
    profile.run("cache_write", &entry.name, || {
        cache::write(entry.key, &image)
    });
    Ok(image)
}

impl ImagePreviewPage {
    pub(super) fn load_directory(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let generation = self.generation;
        let revision = self.revision;
        let profile = Profile::new(generation);
        let queued = profile.clock();
        self.task = Some(cx.spawn_in(w, async move |weak, cx| {
            // Restore small, validated cached resources before exposing the gallery.
            let background_profile = profile.clone();
            let (found, publication) = cx
                .background_spawn(async move {
                    background_profile.handoff(|| {
                        let profile = &background_profile;
                        profile.record("discovery_executor_wait", "gallery", queued);
                        let entries = profile.run("directory_scan", "gallery", paths)?;
                        let mut pending = Vec::new();
                        let items = entries
                            .iter()
                            .enumerate()
                            .map(|(index, entry)| {
                                let document = ImagePreviewDocument::new(entry.name.clone(), revision);
                                if let Some(image) = read_cached(entry, profile) {
                                    document.with_thumbnail(image.clone())
                                        .with_source(ImagePreviewLoad::Ready(image))
                                } else {
                                    pending.push(index);
                                    document.with_source(ImagePreviewLoad::Loading)
                                }
                            })
                            .collect::<Vec<_>>();
                        Ok::<_, String>((entries, pending, items))
                    })
                })
                .await;
            let _ = weak.update_in(cx, |page, w, cx| {
                if page.generation != generation {
                    return;
                }
                profile.record("discovery_publish_wait", "gallery", publication);
                let install = profile.clock();
                match found {
                    Err(error) => {
                        let document = ImagePreviewDocument::new("local-gallery", revision)
                            .with_source(ImagePreviewLoad::Failed(format!("{DIRECTORY}：{error}").into()));
                        let _ = page.preview.update(cx, |s, cx| s.set_document(document, w, cx));
                    }
                    Ok((entries, pending, items)) => {
                        let remaining_thumbnails = pending.len();
                        profile.note(format_args!("gallery_items={} cache_hits={} pending={} thumbnail_workers={THUMBNAIL_WORKERS}", entries.len(), entries.len() - remaining_thumbnails, remaining_thumbnails));
                        page.gallery_session = Some(GallerySession {
                            entries: Arc::new(entries), pending,
                            ready: HashSet::new(), attempted: HashSet::new(),
                            recent: VecDeque::new(), source_running: false,
                            queued_at: profile.clock(), profile: profile.clone(),
                            remaining_thumbnails, pending_proxy: None, pending_source: None,
                        });
                        let _ = page.preview.update(cx, |s, cx| s.set_gallery(items, 0, w, cx));
                        for _ in 0..THUMBNAIL_WORKERS {
                            page.start_thumbnail_worker(generation, w, cx);
                        }
                        page.prioritize_gallery(w, cx);
                        if remaining_thumbnails == 0 {
                            profile.summary("cached_thumbnails_ready");
                        }
                    }
                }
                cx.notify();
                profile.record("gallery_install", "gallery", install);
            });
        }));
    }

    fn start_thumbnail_worker(&mut self, generation: u64, w: &mut Window, cx: &mut Context<Self>) {
        self.gallery_tasks
            .push(cx.spawn_in(w, async move |weak, cx| {
                loop {
                    let job = weak
                        .update_in(cx, |page, _, cx| {
                            if page.generation != generation {
                                return None;
                            }
                            let active = page.preview.read(cx).gallery_index();
                            let session = page.gallery_session.as_mut()?;
                            let index = session.next_thumbnail(active)?;
                            Some((
                                index,
                                session.entries[index].clone(),
                                session.profile.clone(),
                                session.queued_at,
                            ))
                        })
                        .ok()
                        .flatten();
                    let Some((index, entry, profile, queued_at)) = job else {
                        return;
                    };
                    profile.record("thumbnail_queue_wait", &entry.name, queued_at);
                    let executor_queue = profile.clock();
                    let background_profile = profile.clone();
                    let name = entry.name.clone();
                    let (result, publication) = cx
                        .background_spawn(async move {
                            background_profile.record(
                                "thumbnail_executor_wait",
                                &entry.name,
                                executor_queue,
                            );
                            background_profile.handoff(|| thumbnail(&entry, &background_profile))
                        })
                        .await;
                    let current = weak
                        .update_in(cx, |page, w, cx| {
                            if page.generation != generation {
                                return false;
                            }
                            profile.record("thumbnail_publish_wait", &name, publication);
                            let apply = profile.clock();
                            let success = result.is_ok();
                            let high_quality = page
                                .gallery_session
                                .as_ref()
                                .is_some_and(|session| session.ready.contains(&index));
                            page.preview.update(cx, |s, cx| {
                                let mut items = s.gallery().to_vec();
                                let Some(document) = items.get_mut(index) else {
                                    return;
                                };
                                match result {
                                    Ok(image) => {
                                        *document = document.clone().with_thumbnail(image.clone());
                                        if !high_quality {
                                            *document = document
                                                .clone()
                                                .with_source(ImagePreviewLoad::Ready(image));
                                        }
                                    }
                                    Err(error) if document.source().ready().is_none() => {
                                        *document = document
                                            .clone()
                                            .with_source(ImagePreviewLoad::Failed(error.into()));
                                    }
                                    Err(_) => {}
                                }
                                let _ = s.set_gallery(items, s.gallery_index(), w, cx);
                            });
                            profile.record("thumbnail_ui_apply", &name, apply);
                            if let Some(session) = page.gallery_session.as_mut() {
                                if success {
                                    session.report_ready(index, false);
                                }
                                session.remaining_thumbnails -= 1;
                                if session.remaining_thumbnails == 0 {
                                    profile.summary("all_thumbnails_settled");
                                }
                            }
                            true
                        })
                        .unwrap_or(false);
                    if !current {
                        return;
                    }
                }
            }));
    }

    pub(super) fn prioritize_gallery(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let active = self.preview.read(cx).gallery_index();
        let proxy_ready = self
            .preview
            .read(cx)
            .gallery()
            .get(active)
            .is_some_and(|document| document.source().ready().is_some());
        let Some(session) = self.gallery_session.as_mut() else {
            return;
        };
        session.pending_proxy = Some((active, session.profile.clock()));
        session.pending_source = Some((active, session.profile.clock()));
        if proxy_ready {
            session.report_ready(active, session.ready.contains(&active));
        }
        if session.ready.contains(&active) {
            session.touch(active);
        }
        if session.source_running {
            return;
        }
        session.source_running = true;
        let generation = self.generation;
        self.gallery_source_task = Some(cx.spawn_in(w, async move |weak, cx| {
            loop {
                let job = weak
                    .update_in(cx, |page, _, cx| {
                        if page.generation != generation {
                            return None;
                        }
                        let active = page.preview.read(cx).gallery_index();
                        let session = page.gallery_session.as_mut()?;
                        let Some(index) = session.next_source(active) else {
                            session.source_running = false;
                            session.profile.summary("source_prefetch_idle");
                            return None;
                        };
                        if let Some((selected, at)) = session.pending_source
                            && selected == index
                        {
                            session.profile.record(
                                "source_queue_wait",
                                &session.entries[index].name,
                                at,
                            );
                        }
                        Some((
                            index,
                            session.entries[index].clone(),
                            session.profile.clone(),
                        ))
                    })
                    .ok()
                    .flatten();
                let Some((index, entry, profile)) = job else {
                    return;
                };
                let queued = profile.clock();
                let background_profile = profile.clone();
                let name = entry.name.clone();
                let (result, publication) = cx
                    .background_spawn(async move {
                        background_profile.record("source_executor_wait", &entry.name, queued);
                        background_profile
                            .handoff(|| decode(&entry.path, false, entry.name, &background_profile))
                    })
                    .await;
                let current = weak
                    .update_in(cx, |page, w, cx| {
                        if page.generation != generation {
                            return false;
                        }
                        profile.record("source_publish_wait", &name, publication);
                        let apply = profile.clock();
                        let success = result.is_ok();
                        page.publish_source(index, result, w, cx);
                        profile.record("source_ui_apply", &name, apply);
                        if success && let Some(session) = page.gallery_session.as_mut() {
                            session.report_ready(index, true);
                        }
                        true
                    })
                    .unwrap_or(false);
                if !current {
                    return;
                }
            }
        }));
    }

    fn publish_source(
        &mut self,
        index: usize,
        result: Result<ImagePreviewImage, String>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let active = self.preview.read(cx).gallery_index();
        let Some(session) = self.gallery_session.as_mut() else {
            return;
        };
        let mut evicted = Vec::new();
        if result.is_ok() {
            session.ready.insert(index);
            session.touch(index);
            while session.recent.len() > SOURCE_CACHE_SIZE {
                let old = session.recent.pop_front().unwrap();
                if old == active {
                    session.recent.push_back(old);
                    continue;
                }
                session.ready.remove(&old);
                session.attempted.remove(&old);
                evicted.push(old);
            }
        }
        self.preview.update(cx, |s, cx| {
            let mut items = s.gallery().to_vec();
            let Some(document) = items.get_mut(index) else {
                return;
            };
            match result {
                Ok(image) => {
                    *document = document.clone().with_source(ImagePreviewLoad::Ready(image))
                }
                Err(error) if document.source().ready().is_none() => {
                    *document = document
                        .clone()
                        .with_source(ImagePreviewLoad::Failed(error.into()));
                }
                Err(_) => {}
            }
            for old in evicted {
                let fallback = items[old]
                    .thumbnail()
                    .cloned()
                    .map_or(ImagePreviewLoad::Loading, ImagePreviewLoad::Ready);
                items[old] = items[old].clone().with_source(fallback);
            }
            let _ = s.set_gallery(items, s.gallery_index(), w, cx);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "manual source benchmark using the local Showcase image directory"]
    fn profile_local_gallery_source_pipeline() {
        let profile = Profile::new(1);
        let entries = profile.run("directory_scan", "gallery", paths).unwrap();
        for entry in entries.iter().take(1).chain(entries.iter().filter(|entry| {
            matches!(
                entry.name.as_ref(),
                "YueShot_2026-11-07.png" | "SnapClip_2026-11-04.png"
            )
        })) {
            let image = decode(&entry.path, false, entry.name.clone(), &profile).unwrap();
            assert!(image.texture_size().into_iter().all(|edge| edge <= 2048));
        }
        profile.summary("ordinary_long_hdr_sources");
    }
    #[test]
    #[ignore = "manual benchmark using the local Showcase image directory"]
    fn profile_local_gallery_pipeline() {
        use std::{
            sync::atomic::{AtomicUsize, Ordering},
            time::Instant,
        };
        let profile = Profile::new(0);
        let entries = profile.run("directory_scan", "gallery", paths).unwrap();
        let next = AtomicUsize::new(0);
        let started = Instant::now();
        std::thread::scope(|scope| {
            for _ in 0..THUMBNAIL_WORKERS {
                scope.spawn(|| {
                    loop {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some(entry) = entries.get(index) else {
                            break;
                        };
                        let image =
                            decode(&entry.path, true, entry.name.clone(), &profile).unwrap();
                        assert!(image.texture_size().into_iter().all(|edge| edge <= 256));
                    }
                });
            }
        });
        eprintln!(
            "{} thumbnails, {} workers, uncached decode: {:?}",
            entries.len(),
            THUMBNAIL_WORKERS,
            started.elapsed()
        );
        profile.summary("uncached_parallel_decode");
        let started = Instant::now();
        let cached = entries
            .iter()
            .filter(|entry| read_cached(entry, &profile).is_some())
            .count();
        eprintln!(
            "{} / {} cached thumbnails: {:?}",
            cached,
            entries.len(),
            started.elapsed()
        );
        profile.summary("cache_read");
    }
    #[test]
    fn selection_overtakes_pending_thumbnails_and_source_prefetch() {
        let entries = Arc::new(
            (0..50)
                .map(|index| Entry {
                    path: PathBuf::from(format!("{index}.png")),
                    name: index.to_string().into(),
                    key: index as u64,
                })
                .collect(),
        );
        let mut session = GallerySession {
            entries,
            pending: (0..50).collect(),
            ready: HashSet::new(),
            attempted: HashSet::new(),
            recent: VecDeque::new(),
            source_running: false,
            profile: Profile::default(),
            queued_at: None,
            remaining_thumbnails: 50,
            pending_proxy: None,
            pending_source: None,
        };
        assert_eq!(session.next_thumbnail(0), Some(0));
        assert_eq!(session.next_thumbnail(40), Some(40));
        assert_eq!(session.next_source(0), Some(0));
        assert_eq!(session.next_source(40), Some(40));
        session.ready.insert(40);
        assert_eq!(session.next_source(40), Some(41));
        assert_eq!(session.next_source(40), Some(39));
        assert_eq!(session.next_source(40), None);
    }
}
