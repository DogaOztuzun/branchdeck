use crate::error::AppError;
use crate::models::{PtySession, SessionId};
use log::{debug, error, info, trace};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};

pub struct TerminalService {
    sessions: HashMap<SessionId, PtySession>,
}

impl TerminalService {
    #[must_use]
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    pub fn create_session(
        &mut self,
        cwd: &str,
        shell: &str,
        env: &HashMap<String, String>,
    ) -> Result<(SessionId, Box<dyn Read + Send>), AppError> {
        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| AppError::Pty(e.to_string()))?;

        let shell_path = if shell.is_empty() {
            std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
        } else {
            shell.to_string()
        };
        let mut cmd = CommandBuilder::new(&shell_path);
        cmd.cwd(cwd);
        cmd.env("TERM", "xterm-256color");
        for (key, value) in env {
            cmd.env(key, value);
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| AppError::Pty(e.to_string()))?;

        drop(pair.slave);

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| AppError::Pty(e.to_string()))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| AppError::Pty(e.to_string()))?;

        let id = uuid::Uuid::new_v4().to_string();

        let session = PtySession {
            writer,
            master: pair.master,
            child,
        };

        self.sessions.insert(id.clone(), session);

        info!("Created terminal session {id} in {cwd}");

        Ok((id, reader))
    }

    pub fn write_to_session(&mut self, id: &str, data: &[u8]) -> Result<(), AppError> {
        let session = self.sessions.get_mut(id).ok_or_else(|| {
            error!("Write failed: session {id} not found");
            AppError::Pty(format!("Session not found: {id}"))
        })?;

        session.writer.write_all(data).map_err(|e| {
            error!("Write failed for session {id}: {e}");
            AppError::Pty(e.to_string())
        })?;

        trace!("Wrote {} bytes to session {id}", data.len());

        Ok(())
    }

    pub fn resize_session(&mut self, id: &str, rows: u16, cols: u16) -> Result<(), AppError> {
        let session = self.sessions.get(id).ok_or_else(|| {
            error!("Resize failed: session {id} not found");
            AppError::Pty(format!("Session not found: {id}"))
        })?;

        session
            .master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| {
                error!("Resize failed for session {id}: {e}");
                AppError::Pty(e.to_string())
            })?;

        debug!("Resized session {id} to {cols}x{rows}");

        Ok(())
    }

    pub fn close_session(&mut self, id: &str) -> Result<Option<()>, AppError> {
        let session = self.sessions.remove(id);
        if let Some(mut s) = session {
            s.child.kill().map_err(|e| {
                error!("Failed to kill session {id}: {e}");
                AppError::Pty(e.to_string())
            })?;
            info!("Closed terminal session {id}");
            Ok(Some(()))
        } else {
            Ok(None)
        }
    }

    pub fn close_all_sessions(&mut self) {
        let count = self.sessions.len();
        let ids: Vec<String> = self.sessions.keys().cloned().collect();
        for id in ids {
            let _ = self.close_session(&id);
        }
        info!("Closed all {count} terminal sessions");
    }
}
