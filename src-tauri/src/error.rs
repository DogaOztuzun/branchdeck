use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("PTY error: {0}")]
    Pty(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("GitHub error: {0}")]
    GitHub(String),

    #[error("Agent monitoring error: {0}")]
    Agent(String),

    #[error("Task already exists: {0}")]
    TaskAlreadyExists(String),

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Task parse error: {0}")]
    TaskParseError(String),

    #[error("Task watch error: {0}")]
    TaskWatchError(String),

    #[error("Run error: {0}")]
    RunError(String),

    #[error("Sidecar error: {0}")]
    SidecarError(String),
}

// Tauri requires Serialize for error types returned from commands
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
