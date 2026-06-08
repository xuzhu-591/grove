mod cli;
mod interactive;
mod output;

use clap::Parser;
use cli::{CacheAction, Cli, Commands};
use grove_core::config::GroveConfig;
use grove_core::pattern;
use grove_core::worktree::{self, AddOptions};
use std::env;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let cwd = env::current_dir()?;

    match cli.command {
        Commands::List => cmd_list(cli.plain),
        Commands::Add {
            branch,
            create,
            remote,
            no_cache,
        } => cmd_add(cli.plain, branch, create, remote, no_cache, &cwd),
        Commands::Switch { branch } => cmd_switch(cli.plain, branch, &cwd),
        Commands::Remove { branch, force } => cmd_remove(cli.plain, branch, force, &cwd),
        Commands::Cache { action } => cmd_cache(cli.plain, action, &cwd),
    }
}

fn cmd_list(plain: bool) -> anyhow::Result<()> {
    grove_core::git::ensure_git_repo()?;
    let entries = worktree::list_all()?;

    if plain {
        output::print_list_plain(&entries);
    } else {
        output::print_list_pretty(&entries);
    }
    Ok(())
}

fn cmd_add(
    plain: bool,
    branch: Option<String>,
    create: bool,
    remote: bool,
    no_cache: bool,
    cwd: &Path,
) -> anyhow::Result<()> {
    grove_core::git::ensure_git_repo()?;
    let branch = match branch {
        Some(b) => b,
        None => match interactive::add_interactive(cwd)? {
            interactive::AddAction::Existing(b) => b,
            interactive::AddAction::New(b) => {
                let wt_dir = worktree::add(
                    &b,
                    &AddOptions {
                        create: true,
                        remote: false,
                        no_cache,
                    },
                )?;
                output::emit_cd(&wt_dir, plain);
                return Ok(());
            }
            interactive::AddAction::Remote(b) => {
                let wt_dir = worktree::add(
                    &b,
                    &AddOptions {
                        create: false,
                        remote: true,
                        no_cache,
                    },
                )?;
                output::emit_cd(&wt_dir, plain);
                return Ok(());
            }
        },
    };

    let wt_dir = worktree::add(
        &branch,
        &AddOptions {
            create,
            remote,
            no_cache,
        },
    )?;

    if !no_cache {
        let config = GroveConfig::load(cwd)?;
        if !config.cache.rules.is_empty() {
            let rules: Vec<_> = config
                .cache
                .rules
                .iter()
                .filter_map(|r| pattern::compile(r).ok())
                .collect();
            if !rules.is_empty() {
                let main_dir = worktree::main_worktree()?;
                let linked = grove_core::cache::link_cache(&main_dir, &wt_dir, &rules);
                if linked > 0 && !plain {
                    output::info(&format!("Linked {} cache dir(s)", linked));
                }
            }
        }
    }

    output::emit_cd(&wt_dir, plain);
    Ok(())
}

fn cmd_switch(plain: bool, branch: Option<String>, cwd: &Path) -> anyhow::Result<()> {
    grove_core::git::ensure_git_repo()?;
    let branch = match branch {
        Some(b) => b,
        None => interactive::switch_interactive(cwd)?,
    };

    let dir = worktree::find_by_branch(&branch)?;
    if !plain {
        output::info(&format!("-> {}", grove_core::path::short_path(&dir)));
    }
    output::emit_cd(&dir, plain);
    Ok(())
}

fn cmd_remove(plain: bool, branch: Option<String>, force: bool, cwd: &Path) -> anyhow::Result<()> {
    grove_core::git::ensure_git_repo()?;
    let branch = match branch {
        Some(b) => b,
        None => interactive::remove_interactive(cwd)?,
    };

    match worktree::remove(&branch, force) {
        Ok(main_dir) => {
            if !plain {
                output::info(&format!("Removed: {}", branch));
            }
            // If cwd was inside the removed worktree, emit cd to main
            let current = env::current_dir()?;
            if !worktree::is_inside(&current, &main_dir) {
                output::emit_cd(&main_dir, plain);
            }
        }
        Err(e) => {
            output::error(&e.to_string());
            anyhow::bail!("{}", e);
        }
    }
    Ok(())
}

fn cmd_cache(plain: bool, action: Option<CacheAction>, cwd: &Path) -> anyhow::Result<()> {
    grove_core::git::ensure_git_repo()?;
    let action = match action {
        Some(a) => a,
        None => {
            let s = interactive::cache_interactive()?;
            match s.as_str() {
                "link" => CacheAction::Link,
                "status" => CacheAction::Status,
                "unlink" => CacheAction::Unlink,
                _ => anyhow::bail!("unknown cache action"),
            }
        }
    };

    let config = GroveConfig::load(cwd)?;
    let rules: Vec<_> = config
        .cache
        .rules
        .iter()
        .filter_map(|r| pattern::compile(r).ok())
        .collect();

    let main_dir = worktree::main_worktree()?;

    match action {
        CacheAction::Link => {
            let linked = grove_core::cache::link_cache(&main_dir, cwd, &rules);
            if linked > 0 && !plain {
                output::info(&format!(
                    "Linked {} cache dir(s) from {}",
                    linked,
                    grove_core::path::short_path(&main_dir)
                ));
            }
        }
        CacheAction::Unlink => {
            let removed = grove_core::cache::unlink_cache(cwd, &rules);
            if removed > 0 {
                output::info(&format!("Unlinked {} cache dir(s)", removed));
            } else {
                output::info("No cache symlinks to remove");
            }
        }
        CacheAction::Status => {
            if rules.is_empty() {
                output::warn("No cache rules configured (check ~/.config/grove/config.toml and project grove.toml)");
                return Ok(());
            }
            for rule in &rules {
                let status = grove_core::cache::rule_status(rule, &main_dir, cwd);
                match status {
                    grove_core::cache::CacheStatus::Linked { target } => {
                        output::info(&format!("  linked   {} -> {}", rule.raw, target));
                    }
                    grove_core::cache::CacheStatus::Local => {
                        output::warn(&format!("  local    {}", rule.raw));
                    }
                    grove_core::cache::CacheStatus::Missing { source } => {
                        output::error(&format!(
                            "  missing  {} (available in {})",
                            rule.raw, source
                        ));
                    }
                    grove_core::cache::CacheStatus::NotAvailable => {
                        if plain {
                            println!("  N/A      {}", rule.raw);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
