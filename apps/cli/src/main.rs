use anyhow::{bail, Result};
use chrono::Utc;
use clap::{Args, Parser, Subcommand};
use gv_engine::*;
use std::path::PathBuf;

mod edit;

#[derive(Parser)]
#[command(name = "gv", about = "Your PRs and MRs from GitHub, GitLab and Azure DevOps in one place")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Manage projects (named groups of repos).
    #[command(subcommand)]
    Project(ProjectCmd),
    /// List PRs/MRs. Refreshes from providers unless --cached.
    List(ListArgs),
    /// Show one PR/MR with its discussion.
    Show {
        /// Web URL or id like gitlab:gitlab.com/group/proj#12
        target: String,
        #[arg(long)]
        json: bool,
    },
    /// List the files a PR/MR changes with +/- counts (read-only).
    Files {
        target: String,
        /// Print each file's hunks too.
        #[arg(long)]
        diff: bool,
        /// Print the whole content of this path at the head sha.
        #[arg(long)]
        cat: Option<String>,
    },
    /// Post a top-level comment, a reply with --thread, or a line comment with --file and --line.
    Comment {
        target: String,
        body: String,
        #[arg(long, conflicts_with = "file")]
        thread: Option<String>,
        /// File path for an inline comment (needs --line).
        #[arg(long, requires = "line")]
        file: Option<String>,
        #[arg(long, requires = "file")]
        line: Option<u32>,
        /// First line of a multi-line range.
        #[arg(long, requires = "line")]
        start_line: Option<u32>,
        /// old or new (default new).
        #[arg(long, default_value = "new")]
        side: String,
    },
    /// Resolve (or with --off unresolve) an inline thread.
    Resolve {
        target: String,
        thread: String,
        #[arg(long)]
        off: bool,
    },
    /// Edit an existing comment you wrote.
    EditComment { target: String, comment_id: String, body: String },
    /// Submit a review: approve, request-changes, comment or unapprove (GitLab).
    Review {
        target: String,
        /// approve | request-changes | comment | unapprove
        action: String,
        /// Review summary.
        #[arg(short = 'm', long = "message")]
        body: Option<String>,
    },
    #[command(flatten)]
    Edit(edit::EditCmd),
    /// Show the authenticated user for each known host.
    Whoami,
    /// Star or unstar a PR/MR.
    Star {
        target: String,
        #[arg(long)]
        off: bool,
    },
}

#[derive(Subcommand)]
enum ProjectCmd {
    /// Add a project or add a source to an existing one.
    Add {
        name: String,
        /// Local folder to scan for git repos.
        #[arg(long, group = "src")]
        folder: Option<PathBuf>,
        /// Scan depth for --folder.
        #[arg(long, default_value_t = 3)]
        depth: usize,
        /// A repo URL (web or git remote).
        #[arg(long, group = "src")]
        repo: Option<String>,
        /// A group or org as <provider>:<host>/<path>, e.g. gitlab:gitlab.com/my-group/sub.
        #[arg(long, group = "src")]
        group: Option<String>,
    },
    /// List projects and their sources.
    List,
    /// Show the repos a project resolves to.
    Repos { name: String },
    /// Delete a project.
    Rm { name: String },
}

#[derive(Args)]
struct ListArgs {
    #[arg(long)]
    project: Option<String>,
    #[arg(long, conflicts_with = "reviewing")]
    authored: bool,
    #[arg(long)]
    reviewing: bool,
    /// open (default), merged, closed, any
    #[arg(long, default_value = "open")]
    state: String,
    /// Only items waiting on you.
    #[arg(long)]
    needs_you: bool,
    #[arg(long)]
    starred: bool,
    /// Do not refresh; read the local cache only.
    #[arg(long)]
    cached: bool,
    /// Include bot PRs (Renovate, Dependabot, project bots) that are hidden by default.
    #[arg(long)]
    include_bots: bool,
    /// Include PRs inactive longer than the stale threshold (Settings > Filters, default 60 days).
    #[arg(long)]
    include_stale: bool,
    #[arg(long)]
    json: bool,
}

fn parse_group(s: &str) -> Result<Source> {
    let Some((prov, rest)) = s.split_once(':') else { bail!("expected <provider>:<host>/<path>") };
    let Some(provider) = ProviderKind::parse(prov) else { bail!("unknown provider {prov}") };
    let Some((host, path)) = rest.split_once('/') else { bail!("expected <host>/<path>") };
    Ok(Source::Group { provider, host: host.to_ascii_lowercase(), path: path.trim_matches('/').to_string() })
}

fn gv_http_debug() -> bool {
    std::env::var("GV_DEBUG_HTTP").is_ok_and(|v| !v.is_empty() && v != "0")
}

fn ago(t: chrono::DateTime<Utc>) -> String {
    let d = Utc::now() - t;
    if d.num_days() > 0 {
        format!("{}d", d.num_days())
    } else if d.num_hours() > 0 {
        format!("{}h", d.num_hours())
    } else {
        format!("{}m", d.num_minutes().max(0))
    }
}

fn state_str(s: ChangeRequestState) -> &'static str {
    match s {
        ChangeRequestState::Open => "open",
        ChangeRequestState::Draft => "draft",
        ChangeRequestState::Merged => "merged",
        ChangeRequestState::Closed => "closed",
    }
}

fn checks_str(c: &CheckSummary) -> String {
    if c.total == 0 {
        "-".into()
    } else if c.failed > 0 {
        format!("x{}/{}", c.failed, c.total)
    } else if c.pending > 0 {
        format!("~{}/{}", c.passed, c.total)
    } else {
        format!("ok{}", c.total)
    }
}

fn print_row(cr: &ChangeRequest) {
    let flag = if cr.needs_you { "*" } else { " " };
    let reviews: String = cr
        .reviewers
        .iter()
        .map(|r| {
            let m = match r.state {
                ReviewState::Approved => "+",
                ReviewState::ChangesRequested => "!",
                ReviewState::Pending => "?",
                ReviewState::Commented => "c",
            };
            format!("{m}{}", r.user.username)
        })
        .collect::<Vec<_>>()
        .join(",");
    println!(
        "{flag} {:<6} {:>4} {:<7} {:<40} {:<55} @{:<14} {:<6} [{}]",
        cr.id.repo.kind.as_str(),
        ago(cr.updated_at),
        state_str(cr.state),
        format!("{}", cr.id),
        cr.title.chars().take(55).collect::<String>(),
        cr.author.username,
        checks_str(&cr.checks),
        reviews
    );
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let engine = Engine::open_default()?;
    match cli.cmd {
        Cmd::Project(p) => match p {
            ProjectCmd::Add { name, folder, depth, repo, group } => {
                let src = if let Some(f) = folder {
                    Source::Folder { path: std::fs::canonicalize(&f).unwrap_or(f), depth }
                } else if let Some(r) = repo {
                    Source::Repo { url: r }
                } else if let Some(g) = group {
                    parse_group(&g)?
                } else {
                    engine.ensure_project(&name)?;
                    println!("project {name} created (no sources yet)");
                    return Ok(());
                };
                let p = engine.add_source(&name, src)?;
                let repos = engine.resolve_project_repos(p.id).await?;
                println!("project {} now has {} source(s), {} repo(s)", p.name, p.sources.len(), repos.len());
            }
            ProjectCmd::List => {
                for p in engine.projects()? {
                    println!("{} (id {})", p.name, p.id);
                    for s in p.sources {
                        match s {
                            Source::Folder { path, depth } => println!("  folder {} depth {depth}", path.display()),
                            Source::Repo { url } => println!("  repo   {url}"),
                            Source::Group { provider, host, path } => println!("  group  {provider}:{host}/{path}"),
                        }
                    }
                }
            }
            ProjectCmd::Repos { name } => {
                let p = engine.project(&name)?;
                for r in engine.resolve_project_repos(p.id).await? {
                    println!("{r}");
                }
            }
            ProjectCmd::Rm { name } => {
                let p = engine.project(&name)?;
                engine.delete_project(p.id)?;
                println!("deleted {name}");
            }
        },
        Cmd::List(a) => {
            let project = match &a.project {
                Some(n) => Some(engine.project(n)?.id),
                None => None,
            };
            if !a.cached {
                let target = project.map(RefreshTarget::Project).unwrap_or(RefreshTarget::InvolvingMe);
                let rep = engine.refresh(target).await?;
                for (scope, err) in &rep.errors {
                    eprintln!("warning: {scope}: {}", err.lines().next().unwrap_or_default());
                }
                if let Some(t) = rep.paused_until {
                    eprintln!("paused until {} (rate limit); showing cached data", t.with_timezone(&chrono::Local).format("%H:%M"));
                }
                if gv_http_debug() {
                    eprintln!("refresh: {} fetched, {} enriched, {} pruned", rep.fetched, rep.enriched, rep.pruned);
                    eprint!("{}", engine.http_summary());
                }
            }
            let state = match a.state.as_str() {
                "open" | "active" => StateFilter::Active,
                "merged" => StateFilter::Merged,
                "closed" => StateFilter::Closed,
                "any" | "all" => StateFilter::Any,
                s => bail!("unknown state {s}"),
            };
            let involvement = if a.authored {
                Involvement::Authored
            } else if a.reviewing {
                Involvement::Reviewing
            } else {
                Involvement::All
            };
            let l = engine.listing(&ListQuery {
                project,
                filter: Filter { involvement, state },
                starred_only: a.starred,
                needs_you_only: a.needs_you,
                bots: if a.include_bots { BotFilter::off() } else { bot_filter_from_settings() },
                stale_days: stale_days_from_settings(),
                include_stale: a.include_stale,
            })?;
            let (items, hidden) = (l.items, l.hidden_bots);
            if a.json {
                println!("{}", serde_json::to_string_pretty(&items)?);
            } else {
                for cr in &items {
                    print_row(cr);
                }
                let bots = if hidden > 0 { format!(", {hidden} bot PR(s) hidden (--include-bots)") } else { String::new() };
                let stale = if l.stale.is_empty() { String::new() } else { format!(", {} stale hidden (--include-stale)", l.stale.len()) };
                let snoozed = if l.snoozed > 0 { format!(", {} snoozed", l.snoozed) } else { String::new() };
                eprintln!("{} item(s){bots}{stale}{snoozed}; * = needs you", items.len());
            }
        }
        Cmd::Show { target, json } => {
            let id = engine.resolve_id(&target)?;
            let d = engine.detail(&id, true).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&d)?);
                return Ok(());
            }
            let s = &d.summary;
            println!("{}  [{}]{}", s.title, state_str(s.state), if s.needs_you { "  (needs you)" } else { "" });
            println!("{}", s.web_url);
            println!("{} by @{}  {} -> {}", s.id, s.author.username, s.source_branch, s.target_branch);
            println!("created {} ago, updated {} ago", ago(s.created_at), ago(s.updated_at));
            if let (Some(a), Some(dl)) = (s.additions, s.deletions) {
                println!("+{a} -{dl} in {} file(s)", s.changed_files.unwrap_or(0));
            } else if let Some(f) = s.changed_files {
                println!("{f} file(s) changed");
            }
            println!("checks: {} passed, {} failed, {} pending (of {})", s.checks.passed, s.checks.failed, s.checks.pending, s.checks.total);
            for r in &s.reviewers {
                println!("reviewer @{}: {:?}", r.user.username, r.state);
            }
            if !s.labels.is_empty() {
                println!("labels: {}", s.labels.join(", "));
            }
            if let Some(sha) = &d.head_sha {
                println!("head {sha}");
            }
            println!("\n{}\n", if d.description.trim().is_empty() { "(no description)" } else { d.description.trim() });
            for t in &d.threads {
                let res = match t.resolved {
                    Some(true) => " (resolved)",
                    Some(false) => " (unresolved)",
                    None => "",
                };
                let at = t.position.as_ref().map(|p| {
                    let range = p.start_line.map(|s| format!("{s}-")).unwrap_or_default();
                    format!(" at {}:{range}{} ({:?}){}", p.path, p.line, p.side, if t.outdated { " outdated" } else { "" })
                });
                println!("--- thread {}{res}{}", t.id, at.unwrap_or_default());
                for c in &t.comments {
                    let pos = c.position.as_ref().map(|p| format!(" on {}:{}", p.path, p.line.map(|l| l.to_string()).unwrap_or_default())).unwrap_or_default();
                    println!("  @{} {} ago{pos} [{}]", c.author.username, ago(c.created_at), c.id);
                    for l in c.body.lines() {
                        println!("    {l}");
                    }
                }
            }
        }
        Cmd::Files { target, diff, cat } => {
            let id = engine.resolve_id(&target)?;
            if let Some(path) = cat {
                let sha = engine.detail(&id, true).await?.head_sha.unwrap_or_default();
                match engine.file_content(&id, &path, &sha).await? {
                    Some(c) => print!("{c}"),
                    None => bail!("{path} not found at {sha}"),
                }
                return Ok(());
            }
            let files = engine.changed_files(&id).await?;
            let (a, d) = totals(&files);
            println!("{} files  +{a} -{d}", files.len());
            for f in &files {
                let flag = if f.binary { " [binary]" } else if f.too_large { " [too large]" } else { "" };
                let name = if f.old_path != f.new_path { format!("{} -> {}", f.old_path, f.new_path) } else { f.path().to_string() };
                println!("{:?}\t+{} -{}\t{} hunks\t{name}{flag}", f.status, f.additions, f.deletions, f.hunks.len());
                for h in f.hunks.iter().filter(|_| diff) {
                    println!("  {}", h.header);
                    for l in &h.lines {
                        let m = match l.kind { LineKind::Add => '+', LineKind::Del => '-', LineKind::Context => ' ' };
                        let n = |x: Option<u32>| x.map(|x| x.to_string()).unwrap_or_default();
                        println!("  {:>5} {:>5} {m}{}", n(l.old_no), n(l.new_no), l.text);
                    }
                }
            }
        }
        Cmd::Comment { target, body, thread, file, line, start_line, side } => {
            let id = engine.resolve_id(&target)?;
            let c = match (thread, file, line) {
                (Some(t), _, _) => engine.reply_to_thread(&id, &t, &body).await?,
                (None, Some(f), Some(l)) => {
                    let Some(side) = DiffSide::parse(&side) else { bail!("--side must be old or new") };
                    let mut pos = DiffPosition::new(f, side, l);
                    pos.start_line = start_line;
                    engine.detail(&id, true).await?;
                    engine.changed_files(&id).await?;
                    engine.add_line_comment(&id, &pos, &body).await?
                }
                _ => engine.add_comment(&id, &body).await?,
            };
            println!("posted comment {}", c.id);
        }
        Cmd::Resolve { target, thread, off } => {
            let id = engine.resolve_id(&target)?;
            engine.resolve_thread(&id, &thread, !off).await?;
            println!("{} {thread}", if off { "unresolved" } else { "resolved" });
        }
        Cmd::EditComment { target, comment_id, body } => {
            let id = engine.resolve_id(&target)?;
            let c = engine.edit_comment(&id, &comment_id, &body).await?;
            println!("edited comment {}", c.id);
        }
        Cmd::Review { target, action, body } => {
            let Some(act) = ReviewAction::parse(&action) else { bail!("action must be approve, request-changes, comment or unapprove") };
            let id = engine.resolve_id(&target)?;
            let d = engine.detail(&id, true).await?;
            if act == ReviewAction::Comment && body.as_deref().is_none_or(|b| b.trim().is_empty()) && id.repo.kind == ProviderKind::GitLab {
                bail!("a comment review needs -m <body> on GitLab");
            }
            engine.submit_review(&id, act, body.as_deref()).await?;
            println!("{} submitted on {}", act.label(), d.summary.id);
        }
        Cmd::Edit(c) => edit::run(&engine, c).await?,
        Cmd::Whoami => {
            for (k, h) in engine.known_hosts() {
                match engine.whoami(k, &h).await {
                    Ok(u) => println!("{k}:{h} @{}", u.username),
                    Err(e) => println!("{k}:{h} error: {e}"),
                }
            }
        }
        Cmd::Star { target, off } => {
            let id = engine.resolve_id(&target)?;
            engine.set_starred(&id, !off)?;
            println!("{} {}", if off { "unstarred" } else { "starred" }, id);
        }
    }
    Ok(())
}
