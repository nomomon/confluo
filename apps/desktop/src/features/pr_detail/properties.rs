//! Properties card at the top of the Overview (Linear/GitHub sidebar style): reviewers, assignees,
//! pipeline, approvals, labels and branch. People rows open a searchable multi-select of candidate users.

use gpui::{div, prelude::*, AnyElement, ClickEvent, Context, MouseDownEvent, SharedString};
use gv_engine::{PeopleRole, ReviewState, User};

use crate::app::GitViewer;
use crate::state::{Copied, DetailState};
use crate::ui::atoms::{tooltip, Avatar, Badge, BadgeVariant, Icon, IconName, ListItem};
use crate::ui::molecules::{ComboOption, Combobox};
use crate::ui::theme;

impl GitViewer {
    pub(super) fn render_properties(
        &self,
        d: &DetailState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let caps = d
            .detail
            .as_ref()
            .map(|x| x.edit.caps.clone())
            .unwrap_or_default();
        let reviewers: Vec<User> = d.summary.reviewers.iter().map(|r| r.user.clone()).collect();
        let states: Vec<ReviewState> = d.summary.reviewers.iter().map(|r| r.state).collect();
        let assignees = Self::current_people(d, PeopleRole::Assignees);
        let labels = (!d.summary.labels.is_empty()).then(|| {
            let chips = div().flex().flex_wrap().gap(theme::space_1()).children(
                d.summary
                    .labels
                    .iter()
                    .map(|l| Badge::new(l.clone()).variant(BadgeVariant::Outline)),
            );
            row_shell("prop-labels", "Labels", chips.into_any_element(), false)
        });
        div()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .p(theme::LIST_INSET)
            .rounded(theme::LIST_RADIUS)
            .border_1()
            .border_color(theme::border_subtle())
            .bg(self.surfaces().card())
            .child(self.pr_repo_row(d, cx))
            .child(self.people_row(
                d,
                PeopleRole::Reviewers,
                &reviewers,
                Some(&states),
                caps.reviewers,
                cx,
            ))
            .child(self.people_row(
                d,
                PeopleRole::Assignees,
                &assignees,
                None,
                caps.assignees,
                cx,
            ))
            .children(
                self.render_ci(d, "overview", cx)
                    .map(|ci| row_shell("prop-pipeline", "Pipeline", ci, false)),
            )
            .children(
                self.approvals(d)
                    .map(|a| row_shell("prop-approvals", "Approvals", a.into_any_element(), false)),
            )
            .children(labels)
            .child(self.branch_row(d, cx))
    }

    /// Repository and number; right-click renames the repo.
    fn pr_repo_row(&self, d: &DetailState, cx: &mut Context<Self>) -> AnyElement {
        let s = &d.summary;
        let repo = s.id.repo.clone();
        let value = div()
            .flex()
            .items_center()
            .gap(theme::space_1p5())
            .child(crate::features::shared::provider_mark(s.id.repo.kind))
            .child(format!(
                "{} {}",
                self.repo_label(&s.id.repo),
                crate::features::shared::number_label(s)
            ));
        row_shell("prop-repo", "Repository", value.into_any_element(), false)
            .tooltip(tooltip(format!(
                "{}  right-click to rename",
                s.id.repo.path
            )))
            .on_mouse_down(
                gpui::MouseButton::Right,
                cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                    this.open_repo_menu(repo.clone(), ev.position, cx)
                }),
            )
            .into_any_element()
    }

    /// `source -> target` in mono; a click copies the source branch.
    fn branch_row(&self, d: &DetailState, cx: &mut Context<Self>) -> AnyElement {
        let s = &d.summary;
        let branch = s.source_branch.clone();
        let copied = self.copied == Some(Copied::Branch);
        let value = div()
            .font_family(theme::mono_font())
            .text_size(theme::text_xs())
            .truncate()
            .child(SharedString::from(format!(
                "{} -> {}",
                s.source_branch, s.target_branch
            )));
        row_shell("prop-branch", "Branch", value.into_any_element(), true)
            .tooltip(tooltip(if copied {
                "Copied"
            } else {
                "Copy source branch"
            }))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.copy(Copied::Branch, branch.clone(), cx)
            }))
            .into_any_element()
    }

    /// Names with avatars; reviewers also get their state (approved, requested changes, reviewed).
    fn people_value(
        &self,
        users: &[User],
        states: Option<&[ReviewState]>,
        editable: bool,
    ) -> AnyElement {
        if users.is_empty() {
            let text = if editable { "Add" } else { "None" };
            return div()
                .text_color(theme::text_subtle())
                .child(text)
                .into_any_element();
        }
        div()
            .flex()
            .flex_wrap()
            .gap_x(theme::space_3())
            .gap_y(theme::space_1())
            .children(users.iter().enumerate().map(|(i, u)| {
                let name = u
                    .username
                    .strip_prefix("team:")
                    .map(|t| format!("@{t}"))
                    .unwrap_or_else(|| u.username.clone());
                let state = states.and_then(|s| s.get(i)).and_then(|s| match s {
                    ReviewState::Approved => {
                        Some((IconName::CircleCheck, theme::success(), "Approved"))
                    }
                    ReviewState::ChangesRequested => {
                        Some((IconName::CircleX, theme::danger(), "Requested changes"))
                    }
                    ReviewState::Commented => {
                        Some((IconName::MessageCircle, theme::text_muted(), "Reviewed"))
                    }
                    ReviewState::Pending => None,
                });
                let badge = state.map(|(icon, color, tip)| {
                    div()
                        .id(SharedString::from(format!("rev-state-{name}")))
                        .tooltip(tooltip(tip))
                        .child(Icon::new(icon).size(theme::icon_sm()).color(color))
                });
                div()
                    .flex()
                    .items_center()
                    .gap(theme::space_1p5())
                    .child(Avatar::new(name.clone()).image(self.avatar(u.avatar_url.as_deref())))
                    .child(name)
                    .children(badge)
            }))
            .into_any_element()
    }

    fn people_row(
        &self,
        d: &DetailState,
        role: PeopleRole,
        users: &[User],
        states: Option<&[ReviewState]>,
        editable: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = if role == PeopleRole::Reviewers {
            "prop-reviewers"
        } else {
            "prop-assignees"
        };
        let open = d.ui.people == Some(role);
        let row = row_shell(
            id,
            role.label(),
            self.people_value(users, states, editable),
            editable || open,
        );
        let row = if editable {
            row.on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.open_people(role, window, cx)
            }))
        } else {
            row
        };
        let picker = open.then(|| {
            let mut combo = Combobox::new(
                SharedString::from(format!("{id}-picker")),
                self.people_input.clone(),
            )
            .highlight(d.ui.people_hl)
            .loading(d.ui.people_loading && d.ui.people_results.is_empty())
            .empty_text("No one matches")
            .on_toggle(cx.listener(|this, i: &usize, _, cx| this.toggle_person(*i, cx)))
            .on_highlight(cx.listener(|this, i: &usize, _, cx| {
                if let Some(d) = this.detail.as_mut() {
                    d.ui.people_hl = *i;
                }
                cx.notify()
            }))
            .on_dismiss(cx.listener(|this, _: &MouseDownEvent, _, cx| {
                if let Some(d) = this.detail.as_mut() {
                    d.ui.people = None;
                }
                cx.notify()
            }));
            for (u, on) in self.people_options(d, role, cx) {
                combo = combo.option(
                    ComboOption::new(u.username.clone())
                        .detail(u.name.clone())
                        .avatar(self.avatar(u.avatar_url.as_deref()))
                        .selected(on),
                );
            }
            combo
        });
        div()
            .flex()
            .flex_col()
            .child(row)
            .children(picker)
            .into_any_element()
    }
}

/// Label column and value; a `ListItem` when it can be clicked.
fn row_shell(
    id: &'static str,
    label: &'static str,
    value: AnyElement,
    interactive: bool,
) -> gpui::Stateful<gpui::Div> {
    let row = div()
        .id(id)
        .flex()
        .items_center()
        .gap(theme::space_4())
        .min_h(theme::kv_row_h())
        .px(theme::row_pad_x())
        .py(theme::space_1())
        .text_size(theme::text_sm())
        .child(
            div()
                .w(theme::KV_KEY_W)
                .flex_shrink_0()
                .text_color(theme::text_muted())
                .child(label),
        )
        .child(div().flex_1().min_w(theme::space_0()).child(value));
    if interactive {
        row.list_item(false)
    } else {
        row.rounded(theme::LIST_ITEM_RADIUS)
    }
}
