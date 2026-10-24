//! Owned copies of libghostty's OSC 7501 reports, which only live for the
//! callback they're handed to.

use std::str::{Utf8Error, from_utf8};

use libghostty_vt::terminal::{self, ProgramStatusKind, ProgramStatusState};

/// Mirrors GhosttyTerminalProgramStatus; an empty id addresses the root.
#[derive(Clone, Debug)]
pub struct ProgramStatus {
    pub state: ProgramState,
    pub kind: Option<ProgramKind>,
    pub progress: Option<u8>,
    pub id: String,
    pub app: Option<String>,
    pub title: Option<String>,
    pub message: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgramState {
    Idle,
    Working,
    Done,
    Blocked,
    Error,
    Clear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgramKind {
    Permission,
    Question,
    Auth,
}

impl ProgramStatus {
    /// `None` for a state these bindings don't know. An unknown kind is absent.
    pub(crate) fn copy(report: terminal::ProgramStatus<'_>) -> Option<Self> {
        let state = match report.state().ok()? {
            ProgramStatusState::Idle => ProgramState::Idle,
            ProgramStatusState::Working => ProgramState::Working,
            ProgramStatusState::Done => ProgramState::Done,
            ProgramStatusState::Blocked => ProgramState::Blocked,
            ProgramStatusState::Error => ProgramState::Error,
            ProgramStatusState::Clear => ProgramState::Clear,
            _ => return None,
        };
        let kind = match report.kind() {
            Ok(ProgramStatusKind::Permission) => Some(ProgramKind::Permission),
            Ok(ProgramStatusKind::Question) => Some(ProgramKind::Question),
            Ok(ProgramStatusKind::Auth) => Some(ProgramKind::Auth),
            _ => None,
        };
        Some(Self {
            state,
            kind,
            progress: report.progress(),
            id: from_utf8(report.id()).ok()?.to_owned(),
            app: text(report.app()).ok()?,
            title: text(report.title()).ok()?,
            message: text(report.message()).ok()?,
        })
    }
}

/// Ghostty sends text the program left out as empty.
fn text(bytes: &[u8]) -> Result<Option<String>, Utf8Error> {
    if bytes.is_empty() {
        return Ok(None);
    }
    from_utf8(bytes).map(|text| Some(text.to_owned()))
}
