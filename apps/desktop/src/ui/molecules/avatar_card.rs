//! Hover card of an `AvatarGroup`: everyone in it with name, @handle and state. Hidden while another
//! layer (menu, popover) is open, like every tooltip.

use gpui::{div, prelude::*, AnyView, App, Context, Window};

use super::avatar_group::GroupMember;
use crate::ui::atoms::{Avatar, EmptyTooltip as Empty};
use crate::ui::theme;

struct AvatarCard(Vec<GroupMember>);

impl Render for AvatarCard {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(theme::space_1p5())
            .p(theme::space_2())
            .rounded(theme::RADIUS_MD)
            .border_1()
            .border_color(theme::border())
            .bg(theme::overlay())
            .shadow_md()
            .text_size(theme::text_xs())
            .font_family(theme::ui_font())
            .children(self.0.iter().map(|m| {
                let state = m.status.map(|s| {
                    let (_, color, label) = s.look();
                    div().pl(theme::space_2()).text_color(color).child(label)
                });
                div()
                    .flex()
                    .items_center()
                    .gap(theme::space_1p5())
                    .child(Avatar::new(m.name.clone()).image(m.image.clone()))
                    .child(
                        div()
                            .font_weight(theme::WEIGHT_MEDIUM)
                            .text_color(theme::text())
                            .child(m.name.clone()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_color(theme::text_subtle())
                            .child(format!("@{}", m.handle)),
                    )
                    .children(state)
            }))
    }
}

/// Builds the card for GPUI's `.tooltip(..)`; empty while another layer owns the screen.
pub(super) fn card(members: Vec<GroupMember>, window: &mut Window, cx: &mut App) -> AnyView {
    if crate::ui::layer::blocks(window.mouse_position()) {
        return cx.new(|_| Empty).into();
    }
    cx.new(|_| AvatarCard(members)).into()
}
