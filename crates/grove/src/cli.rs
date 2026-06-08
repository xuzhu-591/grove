use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "grove", version, about = "Git worktree manager")]
pub struct Cli {
    #[arg(long, default_value_t = false, global = true)]
    pub plain: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(alias = "ls")]
    List,

    #[command(alias = "new")]
    Add {
        branch: Option<String>,

        #[arg(long, short = 'c')]
        create: bool,

        #[arg(long, short = 'r')]
        remote: bool,

        #[arg(long)]
        no_cache: bool,
    },

    #[command(alias = "cd")]
    Switch { branch: Option<String> },

    #[command(alias = "rm")]
    Remove {
        branch: Option<String>,

        #[arg(long, short = 'f')]
        force: bool,
    },

    Cache {
        #[command(subcommand)]
        action: Option<CacheAction>,
    },
}

#[derive(Subcommand, Debug)]
pub enum CacheAction {
    Link,
    Status,
    Unlink,
}
