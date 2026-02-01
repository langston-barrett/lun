use std::{
    io::{self, IsTerminal as _},
    path::PathBuf,
    time,
};

#[derive(Debug)]
pub(crate) struct Env {
    pub(crate) cwd: PathBuf,
    pub(crate) term_width: Option<u16>,
    pub(crate) start_time: Option<time::Instant>,
}

impl Env {
    pub(crate) fn new() -> Self {
        let term_width = if cfg!(test) || !io::stdout().is_terminal() {
            None
        } else {
            terminal_size::terminal_size().map(|s| s.0.0)
        };
        Self {
            cwd: PathBuf::from("."),
            term_width,
            start_time: Some(time::Instant::now()),
        }
    }

    pub(crate) fn is_tty(&self) -> bool {
        self.term_width.is_some()
    }
}
