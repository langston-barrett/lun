use std::{
    io::{self, IsTerminal as _},
    path::PathBuf,
    time,
};

#[derive(Debug)]
pub(crate) struct Env {
    pub(crate) cwd: PathBuf,
    pub(crate) is_tty: bool,
    pub(crate) start_time: Option<time::Instant>,
}

impl Env {
    pub(crate) fn new() -> Self {
        let is_tty = if cfg!(test) {
            false
        } else {
            io::stderr().is_terminal()
        };
        Self {
            cwd: PathBuf::from("."),
            is_tty,
            start_time: Some(time::Instant::now()),
        }
    }
}
