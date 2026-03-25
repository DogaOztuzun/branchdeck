use crate::error::AppError;
use std::io::Write;
use std::path::Path;

/// Atomically write `content` to `path` using a tmp-then-rename pattern.
///
/// # Errors
///
/// Returns `AppError::Io` if writing the temp file or renaming fails.
#[allow(clippy::disallowed_methods)] // This IS the atomic replacement for std::fs::write
pub fn write_atomic(path: &Path, content: &[u8]) -> Result<(), AppError> {
    let tmp = path.with_extension("tmp");
    if let Some(parent) = tmp.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::File::create(&tmp)?;
    file.write_all(content)?;
    file.sync_all()?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Read and deserialize a file, returning `None` if the file does not exist.
///
/// # Errors
///
/// Returns `AppError::Io` on read errors (other than not-found).
/// Returns `AppError::Config` on deserialization errors.
pub fn read_optional<T: serde::de::DeserializeOwned>(path: &Path) -> Result<Option<T>, AppError> {
    match std::fs::read_to_string(path) {
        Ok(content) => {
            let value: T = serde_json::from_str(&content).map_err(|e| {
                AppError::Config(format!("Failed to parse {}: {e}", path.display()))
            })?;
            Ok(Some(value))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(AppError::Io(e)),
    }
}
