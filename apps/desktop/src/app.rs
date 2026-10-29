//! Root view: sidebar, PR list, detail panel and overlays.

use chrono::{DateTime, Utc};
use gpui::{
    div, img, prelude::*, px, AnyElement, ClickEvent, Context, Div, Entity, Focusable, FontWeight, Hsla, ObjectFit,
    PathPromptOptions, SharedString, Stateful, Window,
};
use gv_engine::{
    ChangeRequest, ChangeRequestDetail, ChangeRequestId, ChangeRequestState, Comment, Engine, Filter, Involvement,
    ListQuery, Project, ProviderKind, RefreshTarget, RepoOverride, RepoRef, ReviewState, Source, StateFilter, Thread,
    User,
};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use crate::bridge;
use crate::ui::molecules::text_input::{InputEvent, TextInput};
use crate::settings::{Appearance, Settings};
use crate::ui::theme;

#[derive(Clone, Copy, PartialEq)]
enum DetailTab {
    Summary,
    Activity,
}

#[derive(Clone, Copy, PartialEq)]
enum SourceKind {
    Folder,
    Repo,
    Group,
}

#[derive(Clone, PartialEq)]
enum Overlay {
    None,
    Settings,
    AddProject,
    Alias,
}

struct DetailState {
    summary: ChangeRequest,
    detail: Option<ChangeRequestDetail>,
    loaded_at: Option<DateTime<Utc>>,
    loading: bool,
    error: Option<String>,
    me: Option<String>,
    tab: DetailTab,
    checks_open: bool,
    reply_to: Option<String>,
    editing: Option<String>,
    posting: bool,
}

pub struct GitViewer {
    engine: Arc<Engine>,
    settings: Settings,
    projects: Vec<Project>,
    selected: Option<i64>,
    tab: Involvement,
    needs_you: bool,
    starred_only: bool,
    items: Vec<ChangeRequest>,
    starred: HashSet<ChangeRequestId>,
    aliases: HashMap<RepoRef, String>,
    refreshing: bool,
    status: Option<String>,
    detail: Option<DetailState>,
    overlay: Overlay,
    picker_open: bool,
    copied: bool,
    // inputs
    search: Entity<TextInput>,
    composer: Entity<TextInput>,
    add_name: Entity<TextInput>,
    add_value: Entity<TextInput>,
    add_kind: SourceKind,
    add_error: Option<String>,
    alias_repo: Option<RepoRef>,
    alias_input: Entity<TextInput>,
    bg_input: Entity<TextInput>,
    tint_input: Entity<TextInput>,
}

fn err_str(e: anyhow::Error) -> String {
    format!("{e:#}")
}

impl GitViewer {
    pub fn new(engine: Arc<Engine>, settings: Settings, cx: &mut Context<Self>) -> Self {
        let search = cx.new(|cx| TextInput::new("Search pull requests", cx));
        let composer = cx.new(|cx| TextInput::new("Leave a comment, Enter to post", cx));
        let add_name = cx.new(|cx| TextInput::new("Project name", cx));
        let add_value = cx.new(|cx| TextInput::new("/path/to/folder", cx));
        let alias_input = cx.new(|cx| TextInput::new("Alias (empty to clear)", cx));
        let bg_input = cx.new(|cx| TextInput::new("/path/to/image.jpg, Enter to apply", cx));
        let tint_input = cx.new(|cx| TextInput::new("#14141c", cx));
        bg_input.update(cx, |i, cx| i.set_text(settings.background_image.clone().unwrap_or_default(), cx));
        tint_input.update(cx, |i, cx| i.set_text(settings.tint.clone(), cx));

        cx.subscribe(&search, |_, _, ev: &InputEvent, cx| {
            if matches!(ev, InputEvent::Changed) {
                cx.notify()
            }
        })
        .detach();
        cx.subscribe(&composer, |this, _, ev: &InputEvent, cx| {
            if matches!(ev, InputEvent::Submit) {
                this.post_comment(cx)
            }
        })
        .detach();
        cx.subscribe(&add_value, |this, _, ev: &InputEvent, cx| {
            if matches!(ev, InputEvent::Submit) {
                this.submit_add_project(cx)
            }
        })
        .detach();
        cx.subscribe(&alias_input, |this, _, ev: &InputEvent, cx| {
            if matches!(ev, InputEvent::Submit) {
                this.save_alias(cx)
            }
        })
        .detach();
        cx.subscribe(&bg_input, |this, input, ev: &InputEvent, cx| {
            if matches!(ev, InputEvent::Submit) {
                let t = input.read(cx).text().trim().to_string();
                this.settings.background_image = (!t.is_empty()).then_some(t);
                this.save_settings(cx);
            }
        })
        .detach();
        cx.subscribe(&tint_input, |this, input, ev: &InputEvent, cx| {
            if matches!(ev, InputEvent::Submit) {
                let t = input.read(cx).text();
                if theme::parse_hex(&t).is_some() {
                    this.settings.tint = t.trim().to_string();
                    this.save_settings(cx);
                }
            }
        })
        .detach();

        let mut this = Self {
            engine,
            settings,
            projects: vec![],
            selected: None,
            tab: Involvement::All,
            needs_you: false,
            starred_only: false,
            items: vec![],
            starred: HashSet::new(),
            aliases: HashMap::new(),
            refreshing: false,
            status: None,
            detail: None,
            overlay: Overlay::None,
            picker_open: false,
            copied: false,
            search,
            composer,
            add_name,
            add_value,
            add_kind: SourceKind::Folder,
            add_error: None,
            alias_repo: None,
            alias_input,
            bg_input,
            tint_input,
        };
        this.reload_projects();
        this.reload();
        this.refresh(cx);
        this
    }

    // ---------- data ----------

    fn reload_projects(&mut self) {
        self.projects = self.engine.projects().unwrap_or_default();
        if let Some(id) = self.selected {
            if !self.projects.iter().any(|p| p.id == id) {
                self.selected = None;
            }
        }
    }

    fn reload(&mut self) {
        let q = ListQuery {
            project: self.selected,
            filter: Filter { involvement: self.tab, state: StateFilter::Active },
            starred_only: self.starred_only,
            needs_you_only: self.needs_you,
        };
        match self.engine.list(&q) {
            Ok(v) => self.items = v,
            Err(e) => self.status = Some(err_str(e)),
        }
        let starred_q = ListQuery {
            project: None,
            filter: Filter { involvement: Involvement::All, state: StateFilter::Any },
            starred_only: true,
            needs_you_only: false,
        };
        self.starred = self.engine.list(&starred_q).unwrap_or_default().into_iter().map(|c| c.id).collect();
        self.aliases.clear();
        let repos: HashSet<RepoRef> = self.items.iter().map(|c| c.id.repo.clone()).collect();
        for r in repos {
            if let Ok(RepoOverride { alias: Some(a), .. }) = self.engine.repo_override(&r) {
                self.aliases.insert(r, a);
            }
        }
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        if self.refreshing {
            return;
        }
        self.refreshing = true;
        let engine = self.engine.clone();
        let target = match self.selected {
            None => RefreshTarget::InvolvingMe,
            Some(id) => RefreshTarget::Project(id),
        };
        let rx = bridge::spawn(async move { engine.refresh(target).await.map_err(err_str) });
        cx.spawn(async move |this, cx| {
            let r = rx.await;
            this.update(cx, |v, cx| {
                v.refreshing = false;
                v.status = match r {
                    Ok(Ok(rep)) if rep.errors.is_empty() => None,
                    Ok(Ok(rep)) => Some(format!(
                        "{} source(s) failed: {}",
                        rep.errors.len(),
                        rep.errors.iter().map(|(l, e)| format!("{l}: {e}")).collect::<Vec<_>>().join("; ")
                    )),
                    Ok(Err(e)) => Some(e),
                    Err(_) => Some("refresh cancelled".into()),
                };
                v.reload();
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn select_project(&mut self, id: Option<i64>, cx: &mut Context<Self>) {
        self.selected = id;
        self.picker_open = false;
        self.detail = None;
        self.reload();
        self.refresh(cx);
    }

    fn open_detail(&mut self, cr: ChangeRequest, cx: &mut Context<Self>) {
        let cached = self.engine.cached_detail(&cr.id).ok().flatten();
        self.detail = Some(DetailState {
            summary: cr,
            detail: cached,
            loaded_at: None,
            loading: false,
            error: None,
            me: None,
            tab: DetailTab::Summary,
            checks_open: false,
            reply_to: None,
            editing: None,
            posting: false,
        });
        self.composer.update(cx, |i, cx| i.set_text("", cx));
        self.load_detail(cx);
    }

    fn load_detail(&mut self, cx: &mut Context<Self>) {
        let Some(d) = self.detail.as_mut() else { return };
        d.loading = true;
        let id = d.summary.id.clone();
        let engine = self.engine.clone();
        let rx = bridge::spawn(async move {
            let me = engine.whoami(id.repo.kind, &id.repo.host).await.ok().map(|u| u.username);
            (engine.detail(&id, true).await.map_err(err_str), me)
        });
        let want = d.summary.id.clone();
        cx.spawn(async move |this, cx| {
            let Ok((r, me)) = rx.await else { return };
            this.update(cx, |v, cx| {
                let Some(d) = v.detail.as_mut().filter(|d| d.summary.id == want) else { return };
                d.loading = false;
                if me.is_some() {
                    d.me = me;
                }
                match r {
                    Ok(det) => {
                        d.summary = det.summary.clone();
                        d.detail = Some(det);
                        d.loaded_at = Some(Utc::now());
                        d.error = None;
                    }
                    Err(e) => d.error = Some(e),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn post_comment(&mut self, cx: &mut Context<Self>) {
        let body = self.composer.read(cx).text().trim().to_string();
        let Some(d) = self.detail.as_mut() else { return };
        if body.is_empty() || d.posting {
            return;
        }
        let id = d.summary.id.clone();
        let me = User::named(d.me.clone().unwrap_or_else(|| "you".into()));
        let optimistic = Comment { id: "pending".into(), author: me, body: body.clone(), created_at: Utc::now(), position: None };
        let reply_to = d.reply_to.take();
        let editing = d.editing.take();
        if let Some(det) = d.detail.as_mut() {
            if let Some(cid) = &editing {
                for c in det.threads.iter_mut().flat_map(|t| t.comments.iter_mut()) {
                    if &c.id == cid {
                        c.body = body.clone();
                    }
                }
            } else if let Some(tid) = &reply_to {
                if let Some(t) = det.threads.iter_mut().find(|t| &t.id == tid) {
                    t.comments.push(optimistic);
                }
            } else {
                det.threads.push(Thread { id: "pending".into(), resolved: None, comments: vec![optimistic] });
            }
        }
        d.posting = true;
        d.tab = DetailTab::Activity;
        self.composer.update(cx, |i, cx| i.set_text("", cx));
        let engine = self.engine.clone();
        let rx = bridge::spawn(async move {
            let r = if let Some(cid) = editing {
                engine.edit_comment(&id, &cid, &body).await
            } else if let Some(tid) = reply_to {
                engine.reply_to_thread(&id, &tid, &body).await
            } else {
                engine.add_comment(&id, &body).await
            };
            r.map(|_| ()).map_err(err_str)
        });
        cx.spawn(async move |this, cx| {
            let r = rx.await.unwrap_or_else(|_| Err("cancelled".into()));
            this.update(cx, |v, cx| {
                if let Some(d) = v.detail.as_mut() {
                    d.posting = false;
                    if let Err(e) = r {
                        d.error = Some(format!("Posting failed: {e}"));
                    }
                }
                v.load_detail(cx);
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn toggle_star(&mut self, id: ChangeRequestId, cx: &mut Context<Self>) {
        let on = !self.starred.contains(&id);
        if let Err(e) = self.engine.set_starred(&id, on) {
            self.status = Some(err_str(e));
        }
        self.reload();
        cx.notify();
    }

    fn save_settings(&mut self, cx: &mut Context<Self>) {
        if let Err(e) = self.settings.save() {
            self.status = Some(format!("saving settings: {e:#}"));
        }
        cx.notify();
    }

    fn submit_add_project(&mut self, cx: &mut Context<Self>) {
        let name = self.add_name.read(cx).text().trim().to_string();
        let value = self.add_value.read(cx).text().trim().to_string();
        if name.is_empty() || value.is_empty() {
            self.add_error = Some("Project name and value are required".into());
            cx.notify();
            return;
        }
        let source = match self.add_kind {
            SourceKind::Folder => {
                let p = if let Some(rest) = value.strip_prefix("~/") {
                    PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(rest)
                } else {
                    PathBuf::from(&value)
                };
                Source::Folder { path: p, depth: 3 }
            }
            SourceKind::Repo => Source::Repo { url: value },
            SourceKind::Group => match parse_group(&value) {
                Some(s) => s,
                None => {
                    self.add_error = Some("Expected provider:host/path, e.g. gitlab:gitlab.com/my-group".into());
                    cx.notify();
                    return;
                }
            },
        };
        match self.engine.add_source(&name, source) {
            Ok(p) => {
                self.add_error = None;
                self.overlay = Overlay::None;
                self.add_value.update(cx, |i, cx| i.set_text("", cx));
                self.reload_projects();
                self.select_project(Some(p.id), cx);
            }
            Err(e) => {
                self.add_error = Some(err_str(e));
                cx.notify();
            }
        }
    }

    fn open_alias(&mut self, repo: Option<RepoRef>, cx: &mut Context<Self>) {
        self.overlay = Overlay::Alias;
        self.set_alias_repo(repo, cx);
    }

    fn set_alias_repo(&mut self, repo: Option<RepoRef>, cx: &mut Context<Self>) {
        let cur = repo.as_ref().and_then(|r| self.engine.repo_override(r).ok()).and_then(|o| o.alias).unwrap_or_default();
        self.alias_input.update(cx, |i, cx| i.set_text(cur, cx));
        self.alias_repo = repo;
        cx.notify();
    }

    fn alias_candidates(&self) -> Vec<RepoRef> {
        let mut v: Vec<RepoRef> = match self.selected {
            Some(id) => self.engine.store().project_repos(id).unwrap_or_default(),
            None => vec![],
        };
        v.extend(self.items.iter().map(|c| c.id.repo.clone()));
        v.sort();
        v.dedup();
        v
    }

    fn save_alias(&mut self, cx: &mut Context<Self>) {
        let Some(repo) = self.alias_repo.clone() else { return };
        let alias = self.alias_input.read(cx).text().trim().to_string();
        let mut o = self.engine.repo_override(&repo).unwrap_or_default();
        o.alias = (!alias.is_empty()).then_some(alias);
        if let Err(e) = self.engine.set_repo_override(&repo, o) {
            self.status = Some(err_str(e));
        }
        self.reload();
        cx.notify();
    }

    fn repo_label(&self, r: &RepoRef) -> String {
        self.aliases.get(r).cloned().unwrap_or_else(|| r.path.clone())
    }

    fn project_name(&self) -> String {
        match self.selected {
            None => "Inbox".into(),
            Some(id) => self.projects.iter().find(|p| p.id == id).map(|p| p.name.clone()).unwrap_or_default(),
        }
    }

    // ---------- rendering ----------

    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let op = self.settings.panel_opacity;
        let mut col = div()
            .flex()
            .flex_col()
            .w(px(220.))
            .h_full()
            .flex_shrink_0()
            .bg(theme::panel(op))
            .border_r_1()
            .border_color(theme::border())
            .pt(px(44.))
            .px_2()
            .gap_0p5()
            .child(section_label("Views"))
            .child(nav_item("nav-inbox", "Inbox", self.selected.is_none()).on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.select_project(None, cx))))
            .child(div().flex().justify_between().items_center().pr_1().child(section_label("Projects")).child(
                icon_btn("add-project", "+").on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.overlay = Overlay::AddProject;
                    let n = this.selected.map(|_| this.project_name()).unwrap_or_default();
                    this.add_name.update(cx, |i, cx| i.set_text(n, cx));
                    cx.notify();
                })),
            ));
        for p in &self.projects {
            let id = p.id;
            col = col.child(
                nav_item(SharedString::from(format!("nav-{id}")), &p.name, self.selected == Some(id))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.select_project(Some(id), cx))),
            );
        }
        if self.projects.is_empty() {
            col = col.child(div().px_2().py_1().text_xs().text_color(theme::faint()).child("No projects yet"));
        }
        col.child(div().flex_1())
            .child(nav_item("nav-alias", "Repo aliases", false).on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.open_alias(None, cx))))
            .child(
                nav_item("nav-settings", "Appearance", self.overlay == Overlay::Settings)
                    .mb_2()
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.overlay = Overlay::Settings;
                        cx.notify()
                    })),
            )
    }

    fn render_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let q = self.search.read(cx).text().to_lowercase();
        let visible: Vec<&ChangeRequest> = self
            .items
            .iter()
            .filter(|c| {
                q.is_empty()
                    || c.title.to_lowercase().contains(&q)
                    || c.id.repo.path.to_lowercase().contains(&q)
                    || c.author.username.to_lowercase().contains(&q)
                    || self.aliases.get(&c.id.repo).is_some_and(|a| a.to_lowercase().contains(&q))
                    || c.id.number.to_string() == q.trim_start_matches(['#', '!'])
            })
            .collect();

        let header = div()
            .flex()
            .items_center()
            .gap_2()
            .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child("Pull requests"))
            .child(div().text_sm().text_color(theme::muted()).child(visible.len().to_string()))
            .child(div().flex_1())
            .child(
                btn("picker", format!("{} \u{25BE}", self.project_name())).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.picker_open = !this.picker_open;
                    cx.notify()
                })),
            )
            .child(
                btn("refresh", if self.refreshing { "Refreshing..." } else { "Refresh" })
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.refresh(cx))),
            );

        let tabs = segmented(
            "tabs",
            &[("Authored", self.tab == Involvement::Authored), ("Reviewing", self.tab == Involvement::Reviewing), ("All", self.tab == Involvement::All)],
            cx,
            |this, i, cx| {
                this.tab = [Involvement::Authored, Involvement::Reviewing, Involvement::All][i];
                this.reload();
                cx.notify();
            },
        );

        let chips = div()
            .flex()
            .gap_2()
            .child(chip("chip-needs", "Needs you", self.needs_you).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.needs_you = !this.needs_you;
                this.reload();
                cx.notify()
            })))
            .child(chip("chip-star", "\u{2605} Starred", self.starred_only).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.starred_only = !this.starred_only;
                this.reload();
                cx.notify()
            })));

        let mut rows = div().id("pr-list").flex().flex_col().flex_1().min_h(px(0.)).overflow_y_scroll().gap_1().pb_4();
        if visible.is_empty() {
            rows = rows.child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .py(px(80.))
                    .gap_1()
                    .child(div().text_color(theme::muted()).child("No open pull requests"))
                    .child(div().text_xs().text_color(theme::faint()).child(if self.refreshing { "Fetching from providers..." } else { "Try another tab or refresh" })),
            );
        }
        let selected_id = self.detail.as_ref().map(|d| d.summary.id.clone());
        for c in visible {
            rows = rows.child(self.render_row(c, selected_id.as_ref() == Some(&c.id), cx));
        }

        let mut col = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(0.))
            .h_full()
            .pt(px(36.))
            .px_4()
            .gap_3()
            .child(header)
            .child(div().flex().items_center().gap_3().child(tabs).child(div().flex_1().child(self.search.clone())))
            .child(chips);
        if let Some(s) = &self.status {
            col = col.child(
                div()
                    .text_xs()
                    .text_color(theme::yellow())
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .bg(theme::yellow().opacity(0.08))
                    .line_clamp(3)
                    .child(s.clone()),
            );
        }
        col.child(rows)
    }

    fn render_row(&self, c: &ChangeRequest, is_selected: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let starred = self.starred.contains(&c.id);
        let cr = c.clone();
        let sid = c.id.clone();
        let reviews = review_summary(c);
        let checks = checks_summary(c);
        div()
            .id(SharedString::from(format!("row-{}", c.id.key())))
            .flex()
            .items_center()
            .gap_3()
            .px_3()
            .py_2()
            .rounded_lg()
            .border_1()
            .border_color(if is_selected { theme::accent().opacity(0.5) } else { theme::border() })
            .bg(if is_selected { theme::selected() } else { theme::card(self.settings.panel_opacity * 0.6) })
            .hover(|s| s.bg(theme::hover()))
            .cursor_pointer()
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.open_detail(cr.clone(), cx)))
            .child(provider_badge(c.id.repo.kind))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w(px(0.))
                    .gap_0p5()
                    .child(div().text_sm().font_weight(FontWeight::MEDIUM).truncate().child(c.title.clone()))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .text_xs()
                            .text_color(theme::muted())
                            .child(div().truncate().child(self.repo_label(&c.id.repo)))
                            .child(format!("#{}", c.id.number))
                            .child(c.author.username.clone())
                            .when(c.needs_you, |d| d.child(div().text_color(theme::accent()).child("needs you"))),
                    ),
            )
            .child(div().flex().flex_col().items_end().gap_0p5().flex_shrink_0().text_xs().child(reviews).child(checks))
            .child(state_pill(c.state))
            .child(div().w(px(56.)).flex_shrink_0().text_xs().text_color(theme::faint()).child(relative_time(c.updated_at)))
            .child(
                div()
                    .id(SharedString::from(format!("star-{}", c.id.key())))
                    .px_1()
                    .text_color(if starred { theme::yellow() } else { theme::faint() })
                    .hover(|s| s.text_color(theme::yellow()))
                    .child(if starred { "\u{2605}" } else { "\u{2606}" })
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        cx.stop_propagation();
                        this.toggle_star(sid.clone(), cx)
                    })),
            )
    }

    fn render_detail(&self, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        let op = self.settings.panel_opacity;
        let s = &d.summary;
        let loaded = match (d.loading, d.loaded_at) {
            (true, _) => "Loading...".to_string(),
            (false, Some(t)) => format!("Loaded {}", relative_time(t)),
            (false, None) => "Cached".into(),
        };
        let url = s.web_url.clone();
        let repo = s.id.repo.clone();
        let cmd = checkout_command(&s.id);
        let cmd2 = cmd.clone();

        let top = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(provider_badge(s.id.repo.kind))
                    .child(div().text_xs().text_color(theme::muted()).truncate().child(format!("{} #{}", self.repo_label(&s.id.repo), s.id.number)))
                    .child(div().flex_1())
                    .child(btn("open-browser", "Open in browser").on_click(cx.listener(move |_, _: &ClickEvent, _, cx| cx.open_url(&url))))
                    .child(btn("alias-btn", "Alias").on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.open_alias(Some(repo.clone()), cx))))
                    .child(icon_btn("close-detail", "\u{2715}").on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.detail = None;
                        cx.notify()
                    }))),
            )
            .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child(s.title.clone()))
            .child(div().flex().gap_2().text_xs().text_color(theme::muted()).child(format!("by {}", s.author.username)).child("\u{00B7}").child(loaded));

        let changes = match (s.changed_files, s.additions, s.deletions) {
            (None, None, None) => "-".to_string(),
            (f, a, dl) => format!("{} files  +{}  -{}", f.unwrap_or(0), a.unwrap_or(0), dl.unwrap_or(0)),
        };
        let info = card(op)
            .child(kv("Status", state_pill(s.state).into_any_element()))
            .child(kv("Review", review_summary(s).into_any_element()))
            .child(kv("Branch", div().text_xs().font_family("Menlo").child(format!("{} -> {}", s.source_branch, s.target_branch)).into_any_element()))
            .child(kv("Changes", div().text_xs().child(changes).into_any_element()));

        let ch = s.checks;
        let mut checks = card(op).child(
            div()
                .id("checks-toggle")
                .flex()
                .items_center()
                .gap_2()
                .cursor_pointer()
                .child(div().text_sm().font_weight(FontWeight::MEDIUM).child("Checks"))
                .child(checks_summary(s))
                .child(div().flex_1())
                .child(div().text_xs().text_color(theme::faint()).child(if d.checks_open { "\u{25BE}" } else { "\u{25B8}" }))
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    if let Some(d) = this.detail.as_mut() {
                        d.checks_open = !d.checks_open;
                    }
                    cx.notify()
                })),
        );
        if d.checks_open {
            checks = checks.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .text_xs()
                    .child(kv("Passed", div().text_color(theme::green()).child(ch.passed.to_string()).into_any_element()))
                    .child(kv("Failed", div().text_color(theme::red()).child(ch.failed.to_string()).into_any_element()))
                    .child(kv("Pending", div().text_color(theme::yellow()).child(ch.pending.to_string()).into_any_element()))
                    .child(kv("Total", div().child(ch.total.to_string()).into_any_element())),
            );
        }

        let checkout = card(op).child(div().text_sm().font_weight(FontWeight::MEDIUM).child("Checkout")).child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.))
                        .px_2()
                        .py_1()
                        .rounded_md()
                        .bg(theme::field())
                        .text_xs()
                        .font_family("Menlo")
                        .truncate()
                        .child(cmd),
                )
                .child(btn("copy-cmd", if self.copied { "Copied" } else { "Copy" }).on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(cmd2.clone()));
                    this.copied = true;
                    cx.notify()
                }))),
        );

        let tabs = segmented(
            "detail-tabs",
            &[
                ("Summary", d.tab == DetailTab::Summary),
                (&format!("Activity {}", d.detail.as_ref().map(|x| x.threads.len()).unwrap_or(0)), d.tab == DetailTab::Activity),
            ],
            cx,
            |this, i, cx| {
                if let Some(d) = this.detail.as_mut() {
                    d.tab = if i == 0 { DetailTab::Summary } else { DetailTab::Activity };
                }
                cx.notify()
            },
        );

        let body: AnyElement = match (&d.detail, d.tab) {
            (None, _) => div().text_sm().text_color(theme::muted()).child(if d.loading { "Loading..." } else { "No detail available" }).into_any_element(),
            (Some(det), DetailTab::Summary) => {
                if det.description.trim().is_empty() {
                    div().text_sm().text_color(theme::faint()).child("No description").into_any_element()
                } else {
                    div().flex().flex_col().gap_1().children(markdown(&det.description)).into_any_element()
                }
            }
            (Some(det), DetailTab::Activity) => self.render_threads(det, d, cx).into_any_element(),
        };

        let mut col = div()
            .id("detail-scroll")
            .flex()
            .flex_col()
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .gap_3()
            .pb_4()
            .child(top)
            .child(info)
            .child(checks)
            .child(checkout);
        if let Some(e) = &d.error {
            col = col.child(div().text_xs().text_color(theme::red()).child(e.clone()));
        }
        col = col.child(tabs).child(card(op).child(body));

        div()
            .flex()
            .flex_col()
            .w(px(540.))
            .flex_shrink_0()
            .h_full()
            .bg(theme::panel(op))
            .border_l_1()
            .border_color(theme::border())
            .pt(px(36.))
            .px_4()
            .gap_3()
            .child(col)
            .child(self.render_composer(d, cx))
    }

    fn render_threads(&self, det: &ChangeRequestDetail, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        let mut col = div().flex().flex_col().gap_3();
        if det.threads.is_empty() {
            col = col.child(div().text_sm().text_color(theme::faint()).child("No comments yet"));
        }
        for t in &det.threads {
            let mut tc = div().flex().flex_col().gap_2().p_2().rounded_md().border_1().border_color(theme::border());
            if let Some(p) = t.comments.first().and_then(|c| c.position.as_ref()) {
                tc = tc.child(
                    div()
                        .text_xs()
                        .font_family("Menlo")
                        .text_color(theme::muted())
                        .child(format!("{}{}", p.path, p.line.map(|l| format!(":{l}")).unwrap_or_default())),
                );
            }
            if t.resolved == Some(true) {
                tc = tc.child(div().text_xs().text_color(theme::green()).child("Resolved"));
            }
            for c in &t.comments {
                let mine = d.me.as_deref().is_some_and(|m| m.eq_ignore_ascii_case(&c.author.username));
                let cid = c.id.clone();
                let cbody = c.body.clone();
                let mut head = div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_xs()
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(c.author.username.clone()))
                    .child(div().text_color(theme::faint()).child(if c.id == "pending" { "posting...".into() } else { relative_time(c.created_at) }))
                    .child(div().flex_1());
                if mine && c.id != "pending" {
                    head = head.child(link(SharedString::from(format!("edit-{}", c.id)), "Edit").on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        if let Some(d) = this.detail.as_mut() {
                            d.editing = Some(cid.clone());
                            d.reply_to = None;
                        }
                        this.composer.update(cx, |i, cx| i.set_text(cbody.clone(), cx));
                        window.focus(&this.composer.focus_handle(cx));
                        cx.notify()
                    })));
                }
                tc = tc.child(div().flex().flex_col().gap_1().child(head).child(div().flex().flex_col().gap_1().children(markdown(&c.body))));
            }
            if t.id != "pending" {
                let tid = t.id.clone();
                tc = tc.child(link(SharedString::from(format!("reply-{}", t.id)), "Reply").on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    if let Some(d) = this.detail.as_mut() {
                        d.reply_to = Some(tid.clone());
                        d.editing = None;
                    }
                    window.focus(&this.composer.focus_handle(cx));
                    cx.notify()
                })));
            }
            col = col.child(tc);
        }
        col
    }

    fn render_composer(&self, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        let mode = if d.editing.is_some() {
            Some("Editing your comment")
        } else if d.reply_to.is_some() {
            Some("Replying to thread")
        } else {
            None
        };
        let mut c = div().flex().flex_col().gap_1().pb_3().pt_2().border_t_1().border_color(theme::border());
        if let Some(m) = mode {
            c = c.child(
                div()
                    .flex()
                    .gap_2()
                    .text_xs()
                    .text_color(theme::muted())
                    .child(m)
                    .child(link("cancel-mode", "Cancel").on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        if let Some(d) = this.detail.as_mut() {
                            d.editing = None;
                            d.reply_to = None;
                        }
                        this.composer.update(cx, |i, cx| i.set_text("", cx));
                        cx.notify()
                    }))),
            );
        }
        c.child(
            div()
                .flex()
                .gap_2()
                .items_center()
                .child(div().flex_1().child(self.composer.clone()))
                .child(btn("post", if d.posting { "Posting..." } else if d.editing.is_some() { "Save" } else { "Comment" }).on_click(
                    cx.listener(|this, _: &ClickEvent, _, cx| this.post_comment(cx)),
                )),
        )
    }

    fn render_picker(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut menu = div()
            .absolute()
            .top(px(70.))
            .right(px(110.))
            .w(px(220.))
            .p_1()
            .flex()
            .flex_col()
            .rounded_lg()
            .border_1()
            .border_color(theme::border())
            .bg(theme::card(0.97))
            .shadow_lg()
            .occlude()
            .child(nav_item("pick-inbox", "Inbox", self.selected.is_none()).on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.select_project(None, cx))));
        for p in &self.projects {
            let id = p.id;
            menu = menu.child(
                nav_item(SharedString::from(format!("pick-{id}")), &p.name, self.selected == Some(id))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.select_project(Some(id), cx))),
            );
        }
        menu
    }

    fn render_overlay(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let (title, content): (&str, AnyElement) = match self.overlay {
            Overlay::None => return None,
            Overlay::Settings => ("Appearance", self.render_settings(cx).into_any_element()),
            Overlay::AddProject => ("Add to project", self.render_add_project(cx).into_any_element()),
            Overlay::Alias => ("Repo aliases", self.render_alias(cx).into_any_element()),
        };
        Some(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(gpui::hsla(0., 0., 0., 0.45))
                .occlude()
                .child(
                    div()
                        .w(px(520.))
                        .max_h(px(620.))
                        .flex()
                        .flex_col()
                        .gap_3()
                        .p_4()
                        .rounded_xl()
                        .border_1()
                        .border_color(theme::border())
                        .bg(theme::card(0.97))
                        .shadow_lg()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .child(div().text_base().font_weight(FontWeight::SEMIBOLD).child(title.to_string()))
                                .child(div().flex_1())
                                .child(icon_btn("close-overlay", "\u{2715}").on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                    this.overlay = Overlay::None;
                                    cx.notify()
                                }))),
                        )
                        .child(content),
                )
                .into_any_element(),
        )
    }

    fn render_settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let s = &self.settings;
        let modes: Vec<(&str, bool)> = Appearance::ALL.iter().map(|a| (a.label(), *a == s.appearance)).collect();
        let appearance = segmented("appearance", &modes, cx, |this, i, cx| {
            this.settings.appearance = Appearance::ALL[i];
            this.save_settings(cx);
        });
        const OPS: [f32; 6] = [0.25, 0.4, 0.55, 0.7, 0.85, 1.0];
        let op_labels: Vec<String> = OPS.iter().map(|o| format!("{}%", (o * 100.) as u32)).collect();
        let ops: Vec<(&str, bool)> = OPS.iter().zip(&op_labels).map(|(o, l)| (l.as_str(), (o - s.panel_opacity).abs() < 0.01)).collect();
        let opacity = segmented("opacity", &ops, cx, |this, i, cx| {
            this.settings.panel_opacity = OPS[i];
            this.save_settings(cx);
        });
        const DIMS: [f32; 5] = [0.0, 0.25, 0.45, 0.65, 0.8];
        let dim_labels: Vec<String> = DIMS.iter().map(|o| format!("{}%", (o * 100.) as u32)).collect();
        let dims: Vec<(&str, bool)> = DIMS.iter().zip(&dim_labels).map(|(o, l)| (l.as_str(), (o - s.background_dim).abs() < 0.01)).collect();
        let dim = segmented("dim", &dims, cx, |this, i, cx| {
            this.settings.background_dim = DIMS[i];
            this.save_settings(cx);
        });
        const TINTS: [&str; 6] = ["#14141c", "#0b1220", "#1a1025", "#0f1a14", "#201410", "#000000"];
        let mut swatches = div().flex().gap_2();
        for t in TINTS {
            let active = s.tint.eq_ignore_ascii_case(t);
            swatches = swatches.child(
                div()
                    .id(SharedString::from(format!("tint-{t}")))
                    .size(px(22.))
                    .rounded_full()
                    .border_2()
                    .border_color(if active { theme::accent() } else { theme::border() })
                    .bg(theme::parse_hex(t).unwrap_or(theme::panel(1.)))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.settings.tint = t.to_string();
                        this.tint_input.update(cx, |i, cx| i.set_text(t, cx));
                        this.save_settings(cx);
                    })),
            );
        }
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(field("Window background", appearance.into_any_element()))
            .child(field("Panel opacity", opacity.into_any_element()))
            .child(field(
                "Background image",
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(div().flex_1().child(self.bg_input.clone()))
                    .child(btn("bg-browse", "Choose...").on_click(cx.listener(|_, _: &ClickEvent, _, cx| {
                        let rx = cx.prompt_for_paths(PathPromptOptions { files: true, directories: false, multiple: false, prompt: Some("Choose".into()) });
                        cx.spawn(async move |this, cx| {
                            if let Ok(Ok(Some(paths))) = rx.await {
                                if let Some(p) = paths.into_iter().next() {
                                    let p = p.to_string_lossy().to_string();
                                    this.update(cx, |v, cx| {
                                        v.bg_input.update(cx, |i, cx| i.set_text(p.clone(), cx));
                                        v.settings.background_image = Some(p);
                                        v.save_settings(cx);
                                    })
                                    .ok();
                                }
                            }
                        })
                        .detach();
                    })))
                    .child(btn("bg-clear", "Clear").on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.settings.background_image = None;
                        this.bg_input.update(cx, |i, cx| i.set_text("", cx));
                        this.save_settings(cx);
                    })))
                    .into_any_element(),
            ))
            .child(field("Image dim", dim.into_any_element()))
            .child(field(
                "Tint",
                div().flex().gap_3().items_center().child(swatches).child(div().w(px(110.)).child(self.tint_input.clone())).into_any_element(),
            ))
            .child(div().text_xs().text_color(theme::faint()).child(format!("Saved to {}", crate::settings::path().display())))
    }

    fn render_add_project(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let kinds = segmented(
            "add-kind",
            &[("Folder", self.add_kind == SourceKind::Folder), ("Repo URL", self.add_kind == SourceKind::Repo), ("Group", self.add_kind == SourceKind::Group)],
            cx,
            |this, i, cx| {
                this.add_kind = [SourceKind::Folder, SourceKind::Repo, SourceKind::Group][i];
                let ph = ["/path/to/folder", "https://github.com/owner/repo", "gitlab:gitlab.com/my-group"][i];
                this.add_value.update(cx, |inp, cx| {
                    inp.set_placeholder(ph);
                    cx.notify()
                });
                cx.notify();
            },
        );
        let mut existing = div().flex().flex_wrap().gap_1();
        for p in &self.projects {
            let n = p.name.clone();
            existing = existing.child(chip(SharedString::from(format!("existing-{}", p.id)), &p.name, false).on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.add_name.update(cx, |i, cx| i.set_text(n.clone(), cx));
            })));
        }
        let mut value_row = div().flex().gap_2().items_center().child(div().flex_1().child(self.add_value.clone()));
        if self.add_kind == SourceKind::Folder {
            value_row = value_row.child(btn("folder-browse", "Choose...").on_click(cx.listener(|_, _: &ClickEvent, _, cx| {
                let rx = cx.prompt_for_paths(PathPromptOptions { files: false, directories: true, multiple: false, prompt: Some("Add".into()) });
                cx.spawn(async move |this, cx| {
                    if let Ok(Ok(Some(paths))) = rx.await {
                        if let Some(p) = paths.into_iter().next() {
                            this.update(cx, |v, cx| v.add_value.update(cx, |i, cx| i.set_text(p.to_string_lossy().to_string(), cx))).ok();
                        }
                    }
                })
                .detach();
            })));
        }
        let mut col = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(field("Project", div().flex().flex_col().gap_1().child(self.add_name.clone()).child(existing).into_any_element()))
            .child(field("Source", kinds.into_any_element()))
            .child(field("Value", value_row.into_any_element()));
        if let Some(e) = &self.add_error {
            col = col.child(div().text_xs().text_color(theme::red()).child(e.clone()));
        }
        col.child(
            div().flex().justify_end().child(btn("add-submit", "Add").on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.submit_add_project(cx)))),
        )
    }

    fn render_alias(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut list = div().id("alias-list").flex().flex_col().max_h(px(300.)).overflow_y_scroll().gap_0p5();
        let cands = self.alias_candidates();
        if cands.is_empty() {
            list = list.child(div().text_sm().text_color(theme::faint()).child("No repos in view. Select a project or refresh first."));
        }
        for r in cands {
            let label = match self.engine.repo_override(&r).ok().and_then(|o| o.alias) {
                Some(a) => format!("{}  ({a})", r.path),
                None => r.path.clone(),
            };
            let active = self.alias_repo.as_ref() == Some(&r);
            list = list.child(
                nav_item(SharedString::from(format!("alias-{r}")), &label, active)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.set_alias_repo(Some(r.clone()), cx))),
            );
        }
        let mut col = div().flex().flex_col().gap_3().child(list);
        if let Some(r) = &self.alias_repo {
            col = col.child(field(
                &format!("Alias for {}", r.path),
                div()
                    .flex()
                    .gap_2()
                    .child(div().flex_1().child(self.alias_input.clone()))
                    .child(btn("alias-save", "Save").on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.save_alias(cx))))
                    .into_any_element(),
            ));
        }
        col
    }
}

impl Render for GitViewer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_background_appearance(self.settings.appearance.to_gpui());
        let tint = theme::parse_hex(&self.settings.tint).unwrap_or(theme::panel(1.));
        let base_alpha = match self.settings.appearance {
            Appearance::Opaque => 1.0,
            Appearance::Blurred => 0.35,
            Appearance::Transparent => 0.55,
        };
        let mut root = div()
            .relative()
            .size_full()
            .font_family(".SystemUIFont")
            .text_color(theme::text())
            .text_sm()
            .bg(Hsla { a: base_alpha, ..tint });
        if let Some(p) = self.settings.background_image.as_ref().filter(|p| std::path::Path::new(p).is_file()) {
            root = root
                .child(img(PathBuf::from(p)).absolute().top_0().left_0().size_full().object_fit(ObjectFit::Cover))
                .child(div().absolute().top_0().left_0().size_full().bg(gpui::hsla(0., 0., 0., self.settings.background_dim)));
        }
        let mut main = div().absolute().top_0().left_0().size_full().flex().child(self.render_sidebar(cx)).child(self.render_list(cx));
        if let Some(d) = &self.detail {
            main = main.child(self.render_detail(d, cx));
        }
        root = root.child(main);
        if self.picker_open {
            root = root.child(self.render_picker(cx));
        }
        if let Some(o) = self.render_overlay(cx) {
            root = root.child(o);
        }
        root
    }
}

// ---------- small components ----------

fn section_label(s: &str) -> Div {
    div().px_2().pt_3().pb_1().text_xs().font_weight(FontWeight::MEDIUM).text_color(theme::faint()).child(s.to_string())
}

fn nav_item(id: impl Into<SharedString>, label: &str, active: bool) -> Stateful<Div> {
    div()
        .id(gpui::ElementId::Name(id.into()))
        .px_2()
        .py_1()
        .rounded_md()
        .text_sm()
        .truncate()
        .cursor_pointer()
        .text_color(if active { theme::text() } else { theme::muted() })
        .when(active, |d| d.bg(theme::selected()))
        .hover(|s| s.bg(theme::hover()))
        .child(label.to_string())
}

fn btn(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Stateful<Div> {
    div()
        .id(gpui::ElementId::Name(id.into()))
        .flex_shrink_0()
        .px_2p5()
        .py_1()
        .rounded_md()
        .border_1()
        .border_color(theme::border())
        .bg(theme::field())
        .text_xs()
        .cursor_pointer()
        .hover(|s| s.bg(theme::hover()))
        .child(label.into())
}

fn icon_btn(id: impl Into<SharedString>, label: &str) -> Stateful<Div> {
    div()
        .id(gpui::ElementId::Name(id.into()))
        .px_1p5()
        .rounded_md()
        .text_color(theme::muted())
        .cursor_pointer()
        .hover(|s| s.bg(theme::hover()).text_color(theme::text()))
        .child(label.to_string())
}

fn link(id: impl Into<SharedString>, label: &str) -> Stateful<Div> {
    div()
        .id(gpui::ElementId::Name(id.into()))
        .text_xs()
        .text_color(theme::accent())
        .cursor_pointer()
        .hover(|s| s.text_color(theme::text()))
        .child(label.to_string())
}

fn chip(id: impl Into<SharedString>, label: &str, active: bool) -> Stateful<Div> {
    div()
        .id(gpui::ElementId::Name(id.into()))
        .px_2p5()
        .py_0p5()
        .rounded_full()
        .border_1()
        .text_xs()
        .cursor_pointer()
        .border_color(if active { theme::accent().opacity(0.6) } else { theme::border() })
        .bg(if active { theme::accent().opacity(0.15) } else { theme::field() })
        .text_color(if active { theme::text() } else { theme::muted() })
        .hover(|s| s.bg(theme::hover()))
        .child(label.to_string())
}

fn segmented(
    id: &str,
    items: &[(&str, bool)],
    cx: &mut Context<GitViewer>,
    on: impl Fn(&mut GitViewer, usize, &mut Context<GitViewer>) + 'static,
) -> Div {
    let on = std::rc::Rc::new(on);
    let mut d = div().flex().flex_shrink_0().p_0p5().gap_0p5().rounded_md().bg(theme::field()).border_1().border_color(theme::border());
    for (i, (label, active)) in items.iter().enumerate() {
        let on = on.clone();
        d = d.child(
            div()
                .id(gpui::ElementId::Name(format!("{id}-{i}").into()))
                .px_2p5()
                .py_0p5()
                .rounded_sm()
                .text_xs()
                .cursor_pointer()
                .text_color(if *active { theme::text() } else { theme::muted() })
                .when(*active, |d| d.bg(theme::selected()))
                .hover(|s| s.text_color(theme::text()))
                .child(label.to_string())
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| on(this, i, cx))),
        );
    }
    d
}

fn card(op: f32) -> Div {
    div().flex().flex_col().gap_2().p_3().rounded_lg().border_1().border_color(theme::border()).bg(theme::card(op * 0.8))
}

fn kv(k: &str, v: AnyElement) -> Div {
    div().flex().items_center().gap_2().child(div().w(px(80.)).text_xs().text_color(theme::muted()).child(k.to_string())).child(v)
}

fn field(label: &str, v: AnyElement) -> Div {
    div().flex().flex_col().gap_1().child(div().text_xs().text_color(theme::muted()).child(label.to_string())).child(v)
}

fn provider_badge(kind: ProviderKind) -> Div {
    let (l, c) = match kind {
        ProviderKind::GitHub => ("GH", gpui::hsla(0., 0., 0.85, 1.)),
        ProviderKind::GitLab => ("GL", gpui::hsla(20. / 360., 0.9, 0.6, 1.)),
        ProviderKind::Azure => ("AZ", gpui::hsla(205. / 360., 0.85, 0.6, 1.)),
    };
    div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .size(px(26.))
        .rounded_md()
        .bg(c.opacity(0.14))
        .text_color(c)
        .text_xs()
        .font_weight(FontWeight::BOLD)
        .child(l)
}

fn state_pill(s: ChangeRequestState) -> Div {
    let (l, c) = match s {
        ChangeRequestState::Open => ("Open", theme::green()),
        ChangeRequestState::Draft => ("Draft", theme::muted()),
        ChangeRequestState::Merged => ("Merged", theme::purple()),
        ChangeRequestState::Closed => ("Closed", theme::red()),
    };
    div().flex_shrink_0().px_2().py_0p5().rounded_full().text_xs().bg(c.opacity(0.15)).text_color(c).child(l)
}

fn review_summary(c: &ChangeRequest) -> Div {
    let n = |s: ReviewState| c.reviewers.iter().filter(|r| r.state == s).count();
    let (a, ch, p) = (n(ReviewState::Approved), n(ReviewState::ChangesRequested), n(ReviewState::Pending));
    let (text, color) = if ch > 0 {
        (format!("{ch} changes requested"), theme::red())
    } else if a > 0 {
        (format!("{a} approved"), theme::green())
    } else if p > 0 {
        (format!("{p} pending review"), theme::yellow())
    } else if c.reviewers.is_empty() {
        ("No reviewers".into(), theme::faint())
    } else {
        ("Commented".into(), theme::muted())
    };
    div().text_xs().text_color(color).child(text)
}

fn checks_summary(c: &ChangeRequest) -> Div {
    let k = c.checks;
    let (text, color) = if k.total == 0 {
        ("No checks".to_string(), theme::faint())
    } else if k.failed > 0 {
        (format!("{}/{} checks failed", k.failed, k.total), theme::red())
    } else if k.pending > 0 {
        (format!("{}/{} checks pending", k.pending, k.total), theme::yellow())
    } else {
        (format!("{}/{} checks passed", k.passed, k.total), theme::green())
    };
    div().text_xs().text_color(color).child(text)
}

fn parse_group(s: &str) -> Option<Source> {
    let (kind, rest) = s.trim().split_once(':')?;
    let provider = ProviderKind::parse(kind)?;
    let (host, path) = rest.trim_start_matches("//").split_once('/')?;
    if host.is_empty() || path.is_empty() {
        return None;
    }
    Some(Source::Group { provider, host: host.to_ascii_lowercase(), path: path.trim_matches('/').to_string() })
}

pub fn checkout_command(id: &ChangeRequestId) -> String {
    let r = &id.repo;
    let n = id.number;
    match r.kind {
        ProviderKind::GitHub if r.host == "github.com" => format!("gh pr checkout {n} --repo {}", r.path),
        ProviderKind::GitHub => format!("gh pr checkout {n} --repo {}/{}", r.host, r.path),
        ProviderKind::GitLab if r.host == "gitlab.com" => format!("glab mr checkout {n} -R {}", r.path),
        ProviderKind::GitLab => format!("glab mr checkout {n} -R {}/{}", r.host, r.path),
        ProviderKind::Azure => format!("az repos pr checkout --id {n}"),
    }
}

pub fn relative_time(t: DateTime<Utc>) -> String {
    let s = (Utc::now() - t).num_seconds().max(0);
    match s {
        0..=59 => "just now".into(),
        60..=3599 => format!("{}m ago", s / 60),
        3600..=86399 => format!("{}h ago", s / 3600),
        86400..=2_591_999 => format!("{}d ago", s / 86400),
        _ => format!("{}mo ago", s / 2_592_000),
    }
}

/// Very small markdown renderer: headings, bullets, code fences, quotes, paragraphs.
fn markdown(src: &str) -> Vec<AnyElement> {
    let mut out = vec![];
    let mut in_code = false;
    let mut code = String::new();
    for line in src.lines() {
        let t = line.trim_end();
        if t.trim_start().starts_with("```") {
            if in_code {
                out.push(
                    div()
                        .p_2()
                        .rounded_md()
                        .bg(theme::field())
                        .text_xs()
                        .font_family("Menlo")
                        .child(std::mem::take(&mut code))
                        .into_any_element(),
                );
            }
            in_code = !in_code;
            continue;
        }
        if in_code {
            if !code.is_empty() {
                code.push('\n');
            }
            code.push_str(t);
            continue;
        }
        let trimmed = t.trim_start();
        let el = if trimmed.is_empty() {
            div().h(px(4.))
        } else if let Some(h) = trimmed.strip_prefix("### ").or_else(|| trimmed.strip_prefix("#### ")) {
            div().pt_1().text_sm().font_weight(FontWeight::SEMIBOLD).child(strip_inline(h))
        } else if let Some(h) = trimmed.strip_prefix("## ") {
            div().pt_1().text_base().font_weight(FontWeight::SEMIBOLD).child(strip_inline(h))
        } else if let Some(h) = trimmed.strip_prefix("# ") {
            div().pt_1().text_lg().font_weight(FontWeight::SEMIBOLD).child(strip_inline(h))
        } else if let Some(b) = trimmed.strip_prefix("- [ ] ").or_else(|| trimmed.strip_prefix("* [ ] ")) {
            div().flex().gap_2().child("\u{2610}").child(strip_inline(b))
        } else if let Some(b) = trimmed.strip_prefix("- [x] ").or_else(|| trimmed.strip_prefix("* [x] ")) {
            div().flex().gap_2().child("\u{2611}").child(strip_inline(b))
        } else if let Some(b) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) {
            let indent = (t.len() - trimmed.len()) as f32 * 6.;
            div().flex().gap_2().pl(px(indent)).child(div().text_color(theme::muted()).child("\u{2022}")).child(strip_inline(b))
        } else if let Some(q) = trimmed.strip_prefix("> ") {
            div().pl_2().border_l_2().border_color(theme::border()).text_color(theme::muted()).child(strip_inline(q))
        } else {
            div().child(strip_inline(trimmed))
        };
        out.push(el.text_sm().into_any_element());
    }
    if in_code && !code.is_empty() {
        out.push(div().p_2().rounded_md().bg(theme::field()).text_xs().font_family("Menlo").child(code).into_any_element());
    }
    out
}

/// Drop the most common inline markers so text reads cleanly.
fn strip_inline(s: &str) -> String {
    let mut s = s.replace("**", "").replace("__", "").replace('`', "");
    // [text](url) -> text (url)
    while let Some(start) = s.find('[') {
        let Some(mid) = s[start..].find("](").map(|i| start + i) else { break };
        let Some(end) = s[mid..].find(')').map(|i| mid + i) else { break };
        let text = s[start + 1..mid].to_string();
        let url = s[mid + 2..end].to_string();
        s.replace_range(start..=end, &format!("{text} ({url})"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_and_checkout() {
        let g = parse_group("gitlab:gitlab.com/a/b").unwrap();
        assert_eq!(g, Source::Group { provider: ProviderKind::GitLab, host: "gitlab.com".into(), path: "a/b".into() });
        assert!(parse_group("nope").is_none());
        let id = ChangeRequestId { repo: RepoRef::new(ProviderKind::GitHub, "github.com", "o/r"), number: 4 };
        assert_eq!(checkout_command(&id), "gh pr checkout 4 --repo o/r");
        let id = ChangeRequestId { repo: RepoRef::new(ProviderKind::GitLab, "gitlab.com", "g/p"), number: 2 };
        assert_eq!(checkout_command(&id), "glab mr checkout 2 -R g/p");
        assert_eq!(strip_inline("see [docs](http://x) **now**"), "see docs (http://x) now");
    }
}
