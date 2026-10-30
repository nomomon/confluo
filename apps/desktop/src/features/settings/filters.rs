//! Filters: hide bot PRs, and the author patterns and branch prefixes that mark them.

use gpui::{div, prelude::*, AnyElement, ClickEvent, Context, Entity};

use crate::app::GitViewer;
use crate::features::shared::layout;
use crate::state::BotList;
use crate::ui::atoms::{Icon, IconName, Switch};
use crate::ui::molecules::text_input::TextInput;
use crate::ui::molecules::{Field, Select};
use crate::ui::theme;
use gv_engine::STALE_CHOICES;

impl GitViewer {
    pub(super) fn render_filters(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let hide = Switch::new("hide-bots", self.settings.hide_bots).on_change(cx.listener(
            |this, on: &bool, _, cx| {
                this.settings.hide_bots = *on;
                this.save_settings(cx);
                this.reload();
            },
        ));
        let stale = STALE_CHOICES
            .iter()
            .fold(Select::new("stale-days"), |sel, d| {
                sel.option(d.map_or("Never".to_string(), |d| format!("{d} days")))
            })
            .selected(
                STALE_CHOICES
                    .iter()
                    .position(|d| *d == self.settings.stale_days)
                    .unwrap_or(1),
            )
            .on_change(cx.listener(|this, i: &usize, _, cx| {
                this.settings.stale_days = STALE_CHOICES[*i];
                this.save_settings(cx);
                this.reload();
            }));
        div()
            .child(self.settings_group(
                None,
                vec![Field::new("Hide PRs inactive for", stale)
                        .description("Older pull requests move to a quiet footer under the list")
                        .horizontal()
                        .into_any_element()],
            ))
            .child(self.settings_group(
                None,
                vec![Field::new("Hide bot pull requests", hide)
                        .description("Renovate, Dependabot and the patterns below")
                        .horizontal()
                        .into_any_element()],
            ))
            .child(self.settings_group(
                Some("Hidden authors (* matches anything)"),
                vec![self.chip_editor(
                    BotList::Authors,
                    &self.settings.hidden_authors,
                    &self.bot_authors_input,
                    cx,
                )],
            ))
            .child(self.settings_group(
                Some("Hidden branch prefixes"),
                vec![self.chip_editor(
                    BotList::Branches,
                    &self.settings.hidden_branch_prefixes,
                    &self.bot_branches_input,
                    cx,
                )],
            ))
    }

    /// Removable chips for each entry above an add input (Enter adds; commas add several).
    fn chip_editor(
        &self,
        list: BotList,
        values: &[String],
        input: &Entity<TextInput>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let chips = div()
            .w_full()
            .flex()
            .flex_wrap()
            .gap(theme::SPACE_1P5)
            .children(values.iter().map(|v| {
                let value = v.clone();
                div()
                    .id(gpui::SharedString::from(format!("chip-{v}")))
                    .flex()
                    .items_center()
                    .gap(theme::SPACE_1)
                    .pl(theme::SPACE_2P5)
                    .pr(theme::SPACE_1P5)
                    .py(theme::SPACE_0P5)
                    .rounded(theme::RADIUS_FULL)
                    .border_1()
                    .border_color(theme::border())
                    .bg(theme::surface())
                    .text_size(theme::text_xs())
                    .font_family(theme::mono_font())
                    .child(v.clone())
                    .child(
                        div()
                            .id(gpui::SharedString::from(format!("chip-x-{v}")))
                            .rounded(theme::RADIUS_FULL)
                            .cursor_pointer()
                            .hover(|s| s.bg(theme::surface_active()))
                            .child(Icon::new(IconName::X).size(theme::ICON_SM))
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.remove_bot_entry(list, &value, cx)
                            })),
                    )
            }));
        // Tag input: the field follows the chips and takes the rest of the line, wrapping when it is short.
        let field = div()
            .flex_1()
            .min_w(layout::CHIP_INPUT_MIN_W)
            .child(input.clone());
        div()
            .w_full()
            .p(theme::SPACE_4)
            .child(chips.items_center().child(field))
            .into_any_element()
    }
}
