//! Author avatars: read from the engine's disk cache or fetched (small, through the host governor)
//! on tokio, then kept decoded in a small in-memory cache.

use gpui::{Context, Image, ImageFormat};
use std::sync::Arc;

use super::bridge;
use crate::app::GitViewer;

/// Upper bound on cached avatars; the cache is cleared when it is reached.
const MAX_AVATARS: usize = 512;

impl GitViewer {
    /// The decoded avatar for `url`, if it has loaded.
    pub(crate) fn avatar(&self, url: Option<&str>) -> Option<Arc<Image>> {
        self.avatars.get(url?).cloned().flatten()
    }

    /// Start fetching avatars of listed authors that are not cached yet.
    pub(crate) fn fetch_avatars(&mut self, cx: &mut Context<Self>) {
        let threads = self.detail.as_ref().and_then(|d| d.detail.as_ref()).map(|d| d.threads.as_slice()).unwrap_or_default();
        let mut wanted: Vec<String> = self
            .items
            .iter()
            .chain(self.detail.as_ref().map(|d| &d.summary))
            .map(|c| &c.author)
            .chain(threads.iter().flat_map(|t| &t.comments).map(|c| &c.author))
            .filter_map(|a| a.avatar_url.clone())
            .filter(|u| !self.avatars.contains_key(u))
            .collect();
        wanted.sort();
        wanted.dedup();
        if wanted.is_empty() {
            return;
        }
        if self.avatars.len() + wanted.len() > MAX_AVATARS {
            self.avatars.clear();
        }
        for url in wanted {
            self.avatars.insert(url.clone(), None);
            let (fetch_url, engine) = (url.clone(), self.engine.clone());
            let rx = bridge::spawn(async move { decode(engine.avatar(&fetch_url).await?) });
            cx.spawn(async move |this, cx| {
                let Ok(Some(img)) = rx.await else { return };
                this.update(cx, |v, cx| {
                    v.avatars.insert(url, Some(Arc::new(img)));
                    cx.notify()
                })
                .ok();
            })
            .detach();
        }
    }
}

fn decode(bytes: Vec<u8>) -> Option<Image> {
    Some(Image::from_bytes(sniff(&bytes)?, bytes))
}

/// Image format from magic bytes; content types from avatar hosts are unreliable.
fn sniff(b: &[u8]) -> Option<ImageFormat> {
    match b {
        [0x89, b'P', b'N', b'G', ..] => Some(ImageFormat::Png),
        [0xFF, 0xD8, ..] => Some(ImageFormat::Jpeg),
        [b'G', b'I', b'F', ..] => Some(ImageFormat::Gif),
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => Some(ImageFormat::Webp),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::sniff;
    use gpui::ImageFormat;

    #[test]
    fn sniffs_png_and_jpeg() {
        assert_eq!(sniff(&[0x89, b'P', b'N', b'G', 0]), Some(ImageFormat::Png));
        assert_eq!(sniff(&[0xFF, 0xD8, 0xFF]), Some(ImageFormat::Jpeg));
        assert_eq!(sniff(b"<html>"), None);
    }
}
