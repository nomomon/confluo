//! One quiet line under the list for what is left out (bots, older PRs), and the "Older" section.

use gpui::{div, prelude::*, AnyElement, ClickEvent, Context, SharedString};

use crate::app::GitViewer;
use crate::ui::molecules::ListSection;
use crate::ui::theme;

fn plural(n: usize, one: &str) -> String {
    format!("{n} {one}{}", if n == 1 { "" } else { "s" })
}

impl GitViewer {
    /// A muted inline link in the footer line.
    fn footer_link(&self, id: &'static str, label: &'static str, f: impl Fn(&mut Self, &mut Context<Self>) + 'static, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id(id)
            .cursor_pointer()
            .text_color(theme::text_muted())
            .hover(|s| s.text_color(theme::text()))
            .child(label)
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| f(this, cx)))
            .into_any_element()
    }

    /// "12 bot PRs and 8 older PRs hidden. Show bots · Show older", one line, only when something is left out.
    pub(super) fn render_quiet_footer(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let (bots, stale) = (self.hidden_bots, self.quiet.stale.len());
        if bots == 0 && stale == 0 {
            return None;
        }
        let mut parts = vec![];
        if bots > 0 && !self.show_bots {
            parts.push(plural(bots, "bot PR"));
        }
        if stale > 0 && !self.quiet.show_stale {
            parts.push(plural(stale, "older PR"));
        }
        let text = if parts.is_empty() { "Showing everything.".to_string() } else { format!("{} hidden.", parts.join(" and ")) };
        let mut line = div().flex().justify_center().items_center().gap(theme::SPACE_1P5).mt(theme::SPACE_8).text_size(theme::text_sm()).text_color(theme::text_subtle()).child(text);
        if bots > 0 {
            let label = if self.show_bots { "Hide bots" } else { "Show bots" };
            line = line.child(self.footer_link("bots-toggle", label, |this, cx| this.toggle_show_bots(cx), cx));
        }
        if bots > 0 && stale > 0 {
            line = line.child("\u{b7}");
        }
        if stale > 0 {
            let label = if self.quiet.show_stale { "Hide older" } else { "Show older" };
            line = line.child(self.footer_link("stale-toggle", label, |this, cx| this.toggle_show_stale(cx), cx));
        }
        Some(line)
    }

    /// PRs inactive past the stale setting, at the bottom once shown from the footer.
    pub(super) fn render_stale_section(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.quiet.show_stale || self.quiet.stale.is_empty() {
            return None;
        }
        let q = self.search.read(cx).text().to_lowercase();
        let items: Vec<_> = self.quiet.stale.iter().filter(|c| self.matches_query(c, &q)).collect();
        let n = items.len();
        let title: SharedString = match self.settings.stale_days {
            Some(d) => format!("Older than {d} days").into(),
            None => "Older".into(),
        };
        Some(
            ListSection::new("section-stale", self.surfaces().card())
                .header(title, n)
                .open(true)
                .on_toggle(cx.listener(|this, _: &ClickEvent, _, cx| this.toggle_show_stale(cx)))
                .children(items.into_iter().map(|c| self.render_row(c, cx).into_any_element()))
                .into_any_element(),
        )
    }
}
