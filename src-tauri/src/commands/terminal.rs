use crate::error::AppError;
use crate::models::PtyEvent;
use crate::services::terminal::TerminalService;
use std::collections::HashMap;
use std::io::Read;
use std::sync::Mutex;
use tauri::ipc::Channel;
use tauri::State;

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn create_terminal_session(
    state: State<'_, Mutex<TerminalService>>,
    cwd: String,
    shell: String,
    env: HashMap<String, String>,
    on_output: Channel<PtyEvent>,
) -> Result<String, AppError> {
    let (session_id, mut reader) = {
        let mut service = state.lock().map_err(|e| AppError::Pty(e.to_string()))?;
        service.create_session(&cwd, &shell, &env)?
    };

    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(n) if n > 0 => {
                    let _ = on_output.send(PtyEvent::Output {
                        bytes: buf[..n].to_vec(),
                    });
                }
                _ => {
                    let _ = on_output.send(PtyEvent::Exit { code: None });
                    break;
                }
            }
        }
    });

    Ok(session_id)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn write_terminal(
    state: State<'_, Mutex<TerminalService>>,
    session_id: String,
    data: Vec<u8>,
) -> Result<(), AppError> {
    let mut service = state.lock().map_err(|e| AppError::Pty(e.to_string()))?;
    service.write_to_session(&session_id, &data)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn resize_terminal(
    state: State<'_, Mutex<TerminalService>>,
    session_id: String,
    rows: u16,
    cols: u16,
) -> Result<(), AppError> {
    let mut service = state.lock().map_err(|e| AppError::Pty(e.to_string()))?;
    service.resize_session(&session_id, rows, cols)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn close_terminal(
    state: State<'_, Mutex<TerminalService>>,
    session_id: String,
) -> Result<(), AppError> {
    let mut service = state.lock().map_err(|e| AppError::Pty(e.to_string()))?;
    service.close_session(&session_id)?;
    Ok(())
}
