//! Author avatars and markdown images: read from the engine's disk cache or fetched (small, through the host governor)
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
        let threads = self
            .detail
            .as_ref()
            .and_then(|d| d.detail.as_ref())
            .map(|d| d.threads.as_slice())
            .unwrap_or_default();
        let mut wanted: Vec<String> = self
            .items
            .iter()
            .chain(self.detail.as_ref().map(|d| &d.summary))
            .map(|c| &c.author)
            .chain(threads.iter().flat_map(|t| &t.comments).map(|c| &c.author))
            .filter_map(|a| a.avatar_url.clone())
            .chain(self.hosts.iter().flatten().filter_map(|h| {
                super::connection::host_avatar_url(h.kind, &h.host, h.user.as_deref()?)
            }))
            .filter(|u| !self.avatars.contains_key(u))
            .collect();
        if let Some(d) = self.detail.as_ref() {
            let det = d.detail.as_ref();
            let people = d
                .summary
                .reviewers
                .iter()
                .map(|r| &r.user)
                .chain(det.into_iter().flat_map(|x| &x.edit.assignees))
                .chain(&d.ui.people_results);
            wanted.extend(
                people
                    .filter_map(|u| u.avatar_url.clone())
                    .filter(|u| !self.avatars.contains_key(u)),
            );
        }
        if let Some(d) = self.detail.as_ref().and_then(|d| d.detail.as_ref()) {
            let bodies = std::iter::once(d.description.as_str()).chain(
                d.threads
                    .iter()
                    .flat_map(|t| &t.comments)
                    .map(|c| c.body.as_str()),
            );
            wanted.extend(
                bodies
                    .flat_map(crate::ui::molecules::markdown::image_urls)
                    .filter(|u| !self.avatars.contains_key(u)),
            );
        }
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
    match sniff(&bytes)? {
        // gpui 0.2.2 passes rasterized SVG pixels through as RGBA while every other format is swapped to
        // BGRA, so a blue logo drew orange. Rasterize here and hand it over as PNG, which gpui swaps right.
        ImageFormat::Svg => Some(Image::from_bytes(ImageFormat::Png, svg_to_png(&bytes)?)),
        f => Some(Image::from_bytes(f, bytes)),
    }
}

/// SVG to PNG at least `SVG_MIN_PX` on its longer side (crisp in avatar sizes on retina screens).
fn svg_to_png(svg: &[u8]) -> Option<Vec<u8>> {
    const SVG_MIN_PX: f32 = 128.;
    let tree = resvg::usvg::Tree::from_data(svg, &resvg::usvg::Options::default()).ok()?;
    let size = tree.size();
    let scale = (SVG_MIN_PX / size.width().max(size.height())).max(1.);
    let (w, h) = ((size.width() * scale).ceil() as u32, (size.height() * scale).ceil() as u32);
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w.max(1), h.max(1))?;
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    pixmap.encode_png().ok()
}

/// Image format from magic bytes; content types from avatar hosts are unreliable.
fn sniff(b: &[u8]) -> Option<ImageFormat> {
    match b {
        [0x89, b'P', b'N', b'G', ..] => Some(ImageFormat::Png),
        [0xFF, 0xD8, ..] => Some(ImageFormat::Jpeg),
        [b'G', b'I', b'F', ..] => Some(ImageFormat::Gif),
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => Some(ImageFormat::Webp),
        _ if b.starts_with(b"<svg")
            || (b.starts_with(b"<?xml") && b.windows(4).take(512).any(|w| w == b"<svg")) =>
        {
            Some(ImageFormat::Svg)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::sniff;
    use gpui::ImageFormat;

    #[test]
    fn svg_keeps_its_colors() {
        // A blue square must stay blue (red and blue channels not swapped) and keep transparency.
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect x="4" y="4" width="8" height="8" fill="#0000ff"/></svg>"##;
        let png = super::svg_to_png(svg).unwrap();
        let p = resvg::tiny_skia::Pixmap::decode_png(&png).unwrap();
        let mid = p.pixel(p.width() / 2, p.height() / 2).unwrap().demultiply();
        assert_eq!((mid.red(), mid.green(), mid.blue(), mid.alpha()), (0, 0, 255, 255));
        assert_eq!(p.pixel(0, 0).unwrap().alpha(), 0);
        assert!(p.width() >= 128);
        assert!(matches!(super::decode(svg.to_vec()).map(|i| i.format), Some(ImageFormat::Png)));
    }

    #[test]
    fn sniffs_png_and_jpeg() {
        assert_eq!(sniff(&[0x89, b'P', b'N', b'G', 0]), Some(ImageFormat::Png));
        assert_eq!(sniff(&[0xFF, 0xD8, 0xFF]), Some(ImageFormat::Jpeg));
        assert_eq!(sniff(b"<html>"), None);
    }
}
