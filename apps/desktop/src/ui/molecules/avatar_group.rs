//! Overlapping avatars (shadcn AvatarGroup): each on a ring in the surrounding surface color, at most
//! `max` shown then a muted "+N", an optional status badge per person, and a hover card listing everyone
//! with name, @handle and state. Hovering one avatar raises it above its neighbours (no layout shift).

use gpui::{div, prelude::*, px, App, ElementId, Hsla, Image, Pixels, SharedString, Window};
use std::sync::Arc;

use crate::ui::atoms::{Avatar, Icon, IconName};
use crate::ui::theme;

/// Per-person state shown as a small badge at the avatar's bottom right.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AvatarStatus {
    Approved,
    ChangesRequested,
    Commented,
}

impl AvatarStatus {
    pub(super) fn look(self) -> (IconName, Hsla, &'static str) {
        match self {
            Self::Approved => (IconName::Check, theme::success(), "Approved"),
            Self::ChangesRequested => (IconName::X, theme::danger(), "Requested changes"),
            Self::Commented => (IconName::MessageCircle, theme::text_subtle(), "Commented"),
        }
    }
}

#[derive(Clone)]
pub struct GroupMember {
    pub name: SharedString,
    pub handle: SharedString,
    pub image: Option<Arc<Image>>,
    pub status: Option<AvatarStatus>,
    /// A team or group: a group icon instead of an initial.
    pub group: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GroupSize {
    Sm,
    Md,
}

/// Avatars drawn and the "+N" remainder for `count` people capped at `max` circles of faces.
pub fn overflow(count: usize, max: usize) -> (usize, usize) {
    let max = max.max(1);
    if count <= max {
        (count, 0)
    } else {
        (max, count - max)
    }
}

#[derive(IntoElement)]
pub struct AvatarGroup {
    id: SharedString,
    members: Vec<GroupMember>,
    max: usize,
    size: GroupSize,
    ring: Hsla,
}

impl AvatarGroup {
    pub fn new(id: impl Into<SharedString>, members: Vec<GroupMember>) -> Self {
        Self {
            id: id.into(),
            members,
            max: 4,
            size: GroupSize::Md,
            ring: theme::overlay(),
        }
    }

    #[allow(dead_code)] // kit API
    pub fn max(mut self, max: usize) -> Self {
        self.max = max;
        self
    }

    pub fn size(mut self, size: GroupSize) -> Self {
        self.size = size;
        self
    }

    /// The surface the group sits on, so the rings cut the overlapping circles apart.
    pub fn ring(mut self, color: Hsla) -> Self {
        self.ring = color;
        self
    }

    /// Ring for a translucent `fill` drawn over `backdrop`: flattened to the opaque color it shows as.
    pub fn ring_over(self, backdrop: Hsla, fill: Hsla) -> Self {
        self.ring(theme::mix(backdrop, fill.opacity(1.), fill.a))
    }
}

fn dims(size: GroupSize) -> (Pixels, Pixels) {
    match size {
        GroupSize::Sm => (theme::space_5(), theme::space_2p5()),
        GroupSize::Md => (theme::space_6(), theme::space_3()),
    }
}

impl RenderOnce for AvatarGroup {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let hovered = window.use_keyed_state(
            ElementId::Name(format!("{}-hover", self.id).into()),
            cx,
            |_, _| None::<usize>,
        );
        let lifted = *hovered.read(cx);
        let (d, badge) = dims(self.size);
        let ring_w = theme::space_0p5();
        let step = d * 0.75;
        let (shown, extra) = overflow(self.members.len(), self.max);
        let circles = shown + usize::from(extra > 0);
        let circle = |i: usize| {
            div()
                .id(ElementId::Name(format!("{}-{i}", self.id).into()))
                .absolute()
                .left(step * i as f32)
                .top(if lifted == Some(i) { -ring_w } else { px(0.) })
                .size(d)
                .p(ring_w)
                .rounded(theme::RADIUS_FULL)
                .bg(self.ring)
        };
        let mut faces: Vec<(usize, gpui::AnyElement)> = Vec::new();
        let mut badges = Vec::new();
        for (i, m) in self.members.iter().take(shown).enumerate() {
            let state = hovered.clone();
            let status = m.status.map(|s| {
                let (icon, color, _) = s.look();
                // Painted above every face (a later neighbour would cover it), at this face's bottom right.
                let lift = if lifted == Some(i) { ring_w } else { px(0.) };
                div()
                    .absolute()
                    .left(step * i as f32 + d - badge + ring_w)
                    .top(d - badge + ring_w - lift)
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(badge)
                    .rounded(theme::RADIUS_FULL)
                    .bg(color)
                    .border_1()
                    .border_color(self.ring)
                    .child(Icon::new(icon).size(badge * 0.7).color(self.ring))
            });
            let el = circle(i)
                .on_hover(move |on, _, cx| {
                    state.update(cx, |h, cx| {
                        *h = if *on { Some(i) } else { h.filter(|&x| x != i) };
                        cx.notify()
                    })
                })
                .child(
                    Avatar::new(m.name.clone())
                        .group(m.group)
                        .size(d - ring_w * 2.)
                        .image(m.image.clone()),
                );
            faces.push((i, el.into_any_element()));
            badges.extend(status);
        }
        // The hovered face paints last, so it sits above both neighbours.
        faces.sort_by_key(|(i, _)| lifted == Some(*i));
        let more = (extra > 0).then(|| {
            circle(shown).child(
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(theme::RADIUS_FULL)
                    .bg(theme::surface_active())
                    .text_size(theme::text_2xs())
                    .text_color(theme::text_muted())
                    .child(format!("+{extra}")),
            )
        });
        let members = self.members.clone();
        div()
            .id(ElementId::Name(self.id.clone()))
            .relative()
            .flex_shrink_0()
            .w(d + step * circles.saturating_sub(1) as f32)
            .h(d)
            .children(faces.into_iter().map(|(_, e)| e))
            .children(more)
            .children(badges)
            .tooltip(move |window, cx| super::avatar_card::card(members.clone(), window, cx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overflow_caps_faces_and_counts_the_rest() {
        assert_eq!(overflow(0, 4), (0, 0));
        assert_eq!(overflow(3, 4), (3, 0));
        assert_eq!(overflow(4, 4), (4, 0));
        assert_eq!(overflow(6, 4), (4, 2));
        assert_eq!(overflow(5, 0), (1, 4), "at least one face");
    }
}
