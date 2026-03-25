use portable_pty::{Child, MasterPty};
use serde::Serialize;
use std::io::Write;

pub type SessionId = String;

pub struct PtySession {
    pub writer: Box<dyn Write + Send>,
    pub master: Box<dyn MasterPty + Send>,
    pub child: Box<dyn Child + Send + Sync>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum PtyEvent {
    Output { bytes: Vec<u8> },
    Exit { code: Option<i32> },
}
