//! Right-click menus of the Code tab, on the shared `ContextMenu`: file tree items (viewed, copy path,
//! view file, open in browser) and diff lines (comment, copy lines, copy link, view file). A right-click
//! on a line outside the selection selects that line first.

use gpui::{prelude::*, AnyElement, ClickEvent, Context, MouseDownEvent, Pixels, Point, Window};

use crate::app::{GitViewer, Layer};
use crate::features::keymap::{self, CopyLink};
use crate::state::{Copied, DetailState};
use crate::ui::atoms::IconName;
use crate::ui::molecules::{ContextMenu, MenuItem, MenuSeparator};

impl GitViewer {
    pub(super) fn open_file_menu(&mut self, fi: usize, at: Point<Pixels>, cx: &mut Context<Self>) {
        self.layer = Some(Layer::FileAt(fi, at));
        cx.notify()
    }

    /// Right-click on a diff line: keep a selection that holds it, else select the line; then the menu.
    pub(super) fn open_lines_menu(
        &mut self,
        fi: usize,
        hi: usize,
        li: usize,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let held = self
            .detail
            .as_ref()
            .and_then(|d| d.code.sel)
            .is_some_and(|s| s.fi == fi && s.hi == hi && s.range.contains(li));
        if !held {
            self.press_diff_line(fi, hi, li, false, cx);
            self.finish_line_drag(window, cx);
        }
        self.layer = Some(Layer::LinesAt(at));
        cx.notify()
    }

    pub(in crate::features::pr_detail) fn render_code_menus(
        &self,
        d: &DetailState,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let dismiss = cx.listener(|this, _: &MouseDownEvent, _, cx| this.dismiss_layer(cx));
        let files = d.code.files.clone().unwrap_or_default();
        match self.layer.clone()? {
            Layer::FileAt(fi, at) => {
                let path = files.get(fi)?.path().to_string();
                let viewed = self.review.viewed.contains(&path);
                let (p1, p2, p3) = (path.clone(), path.clone(), path);
                let menu = ContextMenu::new("file-ctx", at)
                    .on_dismiss(dismiss)
                    .child(
                        MenuItem::new(
                            "file-viewed",
                            if viewed {
                                "Mark as not viewed"
                            } else {
                                "Mark as viewed"
                            },
                        )
                        .icon(IconName::Eye)
                        .on_click(cx.listener(
                            move |this, _: &ClickEvent, _, cx| {
                                this.layer = None;
                                this.toggle_viewed(fi, cx)
                            },
                        )),
                    )
                    .child(
                        MenuItem::new("file-view", "View file")
                            .icon(IconName::File)
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.layer = None;
                                this.open_file_view(p1.clone(), cx)
                            })),
                    )
                    .child(MenuSeparator)
                    .child(
                        MenuItem::new("file-copy-path", "Copy path")
                            .icon(IconName::Copy)
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.layer = None;
                                this.copy(Copied::Path, p2.clone(), cx)
                            })),
                    )
                    .child(
                        MenuItem::new("file-open", "Open in browser")
                            .icon(IconName::ExternalLink)
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.layer = None;
                                if let Some(l) = this.file_web_link(&p3) {
                                    cx.open_url(&l)
                                }
                            })),
                    );
                Some(menu.into_any_element())
            }
            Layer::LinesAt(at) => {
                let s = d.code.sel?;
                let path = files.get(s.fi)?.path().to_string();
                let n = s.range.hi() - s.range.lo() + 1;
                let menu = ContextMenu::new("lines-ctx", at)
                    .on_dismiss(dismiss)
                    .child(
                        MenuItem::new(
                            "lines-comment",
                            if n == 1 {
                                "Comment on line"
                            } else {
                                "Comment on lines"
                            },
                        )
                        .icon(IconName::Message)
                        .shortcut("c")
                        .on_click(cx.listener(
                            |this, _: &ClickEvent, window, cx| {
                                this.layer = None;
                                this.comment_on_selection(window, cx)
                            },
                        )),
                    )
                    .child(MenuSeparator)
                    .child(
                        MenuItem::new(
                            "lines-copy",
                            if n == 1 { "Copy line" } else { "Copy lines" },
                        )
                        .icon(IconName::Copy)
                        .shortcut("\u{2318}C")
                        .on_click(cx.listener(
                            |this, _: &ClickEvent, _, cx| {
                                this.layer = None;
                                this.copy_selection(cx)
                            },
                        )),
                    )
                    .child(
                        MenuItem::new("lines-link", "Copy link")
                            .icon(IconName::Link)
                            .shortcut(keymap::kbd::<CopyLink>())
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.layer = None;
                                this.copy_selection_link(cx)
                            })),
                    )
                    .child(
                        MenuItem::new("lines-view", "View file")
                            .icon(IconName::File)
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.layer = None;
                                this.open_file_view(path.clone(), cx)
                            })),
                    );
                Some(menu.into_any_element())
            }
            _ => None,
        }
    }
}
