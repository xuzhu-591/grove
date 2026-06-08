use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GroveError {
    #[error("not a git repository")]
    NotGitRepo,

    #[error("no origin remote configured")]
    NoOriginRemote,

    #[error("worktree not found for branch '{0}'")]
    WorktreeNotFound(String),

    #[error("cannot remove main worktree")]
    CannotRemoveMain,

    #[error("worktree has uncommitted changes:\n{0}")]
    UncommittedChanges(String),

    #[error("worktree has unpushed commits:\n{0}")]
    UnpushedCommits(String),

    #[error("git command failed: {0}")]
    GitError(String),

    #[error("invalid cache pattern '{0}': {1}")]
    InvalidCachePattern(String, String),

    #[error("config file error ({0}): {1}")]
    ConfigError(PathBuf, String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type GroveResult<T> = Result<T, GroveError>;
