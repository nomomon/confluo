//! `gv edit|reviewers|assignees|draft|close|reopen|merge|checks`.

use anyhow::{bail, Result};
use clap::Subcommand;
use gv_engine::*;

#[derive(Subcommand)]
pub enum EditCmd {
    /// Change the title and/or description.
    Edit {
        target: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        body: Option<String>,
    },
    /// Add or remove reviewers: `+ada -bob`.
    Reviewers {
        target: String,
        #[arg(allow_hyphen_values = true, num_args = 0..)]
        changes: Vec<String>,
        /// List users matching this text who can be added (read-only).
        #[arg(long)]
        search: Option<String>,
    },
    /// Add or remove assignees: `+ada -bob`.
    Assignees {
        target: String,
        #[arg(allow_hyphen_values = true, num_args = 0..)]
        changes: Vec<String>,
        /// List users matching this text who can be added (read-only).
        #[arg(long)]
        search: Option<String>,
    },
    /// Mark as draft (`on`) or ready for review (`off`).
    Draft { target: String, state: String },
    /// Add (or with --remove, remove) my reaction: +1 -1 laugh hooray confused heart rocket eyes.
    React {
        target: String,
        kind: String,
        #[arg(long)]
        remove: bool,
        /// Comment id; the PR itself when omitted.
        #[arg(long)]
        comment: Option<String>,
    },
    /// Close a PR/MR.
    Close { target: String },
    /// Reopen a closed PR/MR.
    Reopen { target: String },
    /// Merge (merge commit by default).
    Merge {
        target: String,
        #[arg(long, conflicts_with = "rebase")]
        squash: bool,
        #[arg(long)]
        rebase: bool,
        /// GitLab: delete the source branch.
        #[arg(long)]
        delete_branch: bool,
        /// GitLab: merge when the pipeline succeeds.
        #[arg(long)]
        auto: bool,
    },
    /// List the checks of the head commit; --log prints a failed check's log tail.
    Checks {
        target: String,
        #[arg(long)]
        log: Option<String>,
    },
}

fn print_people(role: PeopleRole, users: &[User]) {
    let list = users
        .iter()
        .map(|u| format!("@{}", u.username))
        .collect::<Vec<_>>()
        .join(", ");
    println!(
        "{}: {}",
        role.label(),
        if list.is_empty() { "none".into() } else { list }
    );
}

async fn people(
    engine: &Engine,
    target: &str,
    role: PeopleRole,
    changes: &[String],
    search: Option<String>,
) -> Result<()> {
    let id = engine.resolve_id(target)?;
    if let Some(q) = search {
        for u in engine.search_users(&id, &q).await? {
            println!("@{}\t{}", u.username, u.name.unwrap_or_default());
        }
        return Ok(());
    }
    let d = engine.detail(&id, true).await?;
    if changes.is_empty() {
        print_people(role, &Engine::people(&d, role));
        return Ok(());
    }
    let desired = engine.people_from_args(&id, role, changes).await?;
    engine.set_people(&id, role, desired.clone()).await?;
    print_people(role, &desired);
    Ok(())
}

pub async fn run(engine: &Engine, cmd: EditCmd) -> Result<()> {
    match cmd {
        EditCmd::Edit {
            target,
            title,
            body,
        } => {
            if title.is_none() && body.is_none() {
                bail!("pass --title and/or --body");
            }
            let id = engine.resolve_id(&target)?;
            engine.detail(&id, true).await?;
            engine
                .update_change_request(&id, title.as_deref(), body.as_deref())
                .await?;
            println!("updated {id}");
        }
        EditCmd::Reviewers {
            target,
            changes,
            search,
        } => people(engine, &target, PeopleRole::Reviewers, &changes, search).await?,
        EditCmd::Assignees {
            target,
            changes,
            search,
        } => people(engine, &target, PeopleRole::Assignees, &changes, search).await?,
        EditCmd::Draft { target, state } => {
            let draft = match state.as_str() {
                "on" | "true" | "yes" => true,
                "off" | "false" | "no" | "ready" => false,
                _ => bail!("state must be on or off"),
            };
            let id = engine.resolve_id(&target)?;
            engine.detail(&id, true).await?;
            engine.set_draft(&id, draft).await?;
            println!(
                "{id} {}",
                if draft {
                    "marked as draft"
                } else {
                    "ready for review"
                }
            );
        }
        EditCmd::React {
            target,
            kind,
            remove,
            comment,
        } => {
            let id = engine.resolve_id(&target)?;
            let Some(k) = ReactionKind::parse(&kind) else {
                bail!("unknown reaction {kind}")
            };
            engine.detail(&id, false).await?;
            engine.react(&id, comment.as_deref(), k, !remove).await?;
            let d = engine.detail(&id, false).await?;
            for g in &d.reactions {
                println!(
                    "{} {} {}{}",
                    g.kind.github_rest(),
                    g.count,
                    g.users.join(","),
                    if g.me { " (me)" } else { "" }
                );
            }
        }
        EditCmd::Close { target } => set_open(engine, &target, false).await?,
        EditCmd::Reopen { target } => set_open(engine, &target, true).await?,
        EditCmd::Merge {
            target,
            squash,
            rebase,
            delete_branch,
            auto,
        } => {
            let id = engine.resolve_id(&target)?;
            engine.detail(&id, false).await?;
            let method = if squash && id.repo.kind == ProviderKind::GitHub {
                MergeMethod::Squash
            } else if rebase {
                MergeMethod::Rebase
            } else {
                MergeMethod::Merge
            };
            let opts = MergeOptions {
                method,
                squash,
                delete_branch,
                when_pipeline_succeeds: auto,
            };
            engine.merge(&id, &opts).await?;
            println!(
                "{} {id}",
                if auto {
                    "set to merge when the pipeline succeeds:"
                } else {
                    "merged"
                }
            );
        }
        EditCmd::Checks { target, log } => {
            let id = engine.resolve_id(&target)?;
            let d = engine.detail(&id, true).await?;
            let now = chrono::Utc::now();
            println!("{}", checks_sentence(&d.checks, now));
            for r in d.checks.sorted() {
                let dur = r
                    .duration_secs(now)
                    .map(format_duration)
                    .unwrap_or_default();
                println!(
                    "{:?}\t{}\t{}\t{dur}\t{}",
                    r.status,
                    r.name,
                    r.stage.clone().unwrap_or_default(),
                    r.url.clone().unwrap_or_default()
                );
            }
            if let Some(name) = log {
                let Some(run) = d
                    .checks
                    .runs
                    .iter()
                    .find(|r| r.name == name || r.id == name)
                else {
                    bail!("no check {name}")
                };
                for l in engine.check_log(&id, run).await? {
                    println!("  {l}");
                }
            }
        }
    }
    Ok(())
}

async fn set_open(engine: &Engine, target: &str, open: bool) -> Result<()> {
    let id = engine.resolve_id(target)?;
    engine.detail(&id, true).await?;
    engine.set_open(&id, open).await?;
    println!("{} {id}", if open { "reopened" } else { "closed" });
    Ok(())
}
