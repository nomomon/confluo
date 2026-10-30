//! `gv auth set|rm|list`: personal access tokens in the system keychain.

use anyhow::{anyhow, bail, Result};
use clap::Subcommand;
use gv_engine::{Account, Engine};
use std::io::{BufRead, IsTerminal};

#[derive(Subcommand)]
pub enum AuthCmd {
    /// Store a token read from stdin (hidden when typed), e.g. `gv auth set azure:dev.azure.com/myorg`.
    Set {
        /// <provider>:<host>[/org], provider github, gitlab or azure; the org is Azure only.
        account: String,
        /// Store without checking it with a whoami call.
        #[arg(long)]
        no_test: bool,
    },
    /// Remove a stored token.
    Rm { account: String },
    /// Known accounts and where their token comes from (env, keychain, cli). Never prints tokens.
    List,
}

fn parse(s: &str) -> Result<Account> {
    Account::parse(s).ok_or_else(|| {
        anyhow!(
            "expected <provider>:<host>[/org], e.g. github:github.com or azure:dev.azure.com/myorg"
        )
    })
}

/// One token line from the terminal (no echo) or from piped stdin.
fn read_token() -> Result<String> {
    let t = if std::io::stdin().is_terminal() {
        rpassword::prompt_password("Token: ")?
    } else {
        let mut line = String::new();
        std::io::stdin().lock().read_line(&mut line)?;
        line
    };
    let t = t.trim().to_string();
    if t.is_empty() {
        bail!("no token on stdin");
    }
    Ok(t)
}

pub async fn run(engine: &Engine, cmd: AuthCmd) -> Result<()> {
    match cmd {
        AuthCmd::Set { account, no_test } => {
            let a = parse(&account)?;
            let token = read_token()?;
            if !no_test {
                let u = engine
                    .test_token(&a, &token)
                    .await
                    .map_err(|e| anyhow!("token rejected by {}: {e}", a.host))?;
                println!("Signed in as @{}", u.username);
            }
            engine.set_token(&a, &token).await?;
            println!("Stored token for {a} in the keychain");
        }
        AuthCmd::Rm { account } => {
            let a = parse(&account)?;
            engine.remove_token(&a).await?;
            println!("Removed token for {a}");
        }
        AuthCmd::List => {
            for (a, origin) in engine.auth_sources().await {
                let src = origin.map(|o| o.label()).unwrap_or("signed out");
                println!("{:<44} {src}", a.name());
            }
        }
    }
    Ok(())
}
