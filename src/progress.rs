use std::cmp;
use std::io::Write;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub(crate) enum Format {
    No,
    Terminal,
    Newline,
}

const TERMINAL_RATE_LIMIT: Duration = Duration::from_millis(100);

pub(crate) struct Progress<'a, W: Write + ?Sized> {
    format: Format,
    pub(crate) completed: usize,
    pub(crate) total: Option<usize>,
    out: &'a mut W,
    done: bool,
    last_write: Option<Instant>,
}

impl<'a, W: Write + ?Sized> Progress<'a, W> {
    pub(crate) fn new(format: Format, total: Option<usize>, out: &'a mut W) -> Self {
        Self {
            format,
            completed: 0,
            total,
            out,
            done: false,
            last_write: None,
        }
    }

    pub(crate) fn increment(&mut self) {
        self.completed += 1;
    }

    fn should_write(&mut self) -> bool {
        if !matches!(self.format, Format::Terminal) {
            return true;
        }
        let dominated = self
            .last_write
            .is_some_and(|t| t.elapsed() < TERMINAL_RATE_LIMIT);
        if !dominated {
            self.last_write = Some(Instant::now());
        }
        !dominated
    }

    /// Report progress at the current completed position.
    pub(crate) fn write(&mut self, msg: &str) {
        if self.should_write() {
            report_line(self.format, self.completed, self.total, msg, self.out);
        }
    }

    /// Report progress at completed+1 (showing the item being worked on).
    pub(crate) fn report(&mut self, msg: &str) {
        if self.should_write() {
            report_line(self.format, self.completed + 1, self.total, msg, self.out);
        }
    }

    pub(crate) fn fail(&mut self, msg: &[u8]) {
        let prefix = match self.format {
            Format::Terminal => b"\n".as_slice(),
            Format::Newline | Format::No => b"".as_slice(),
        };
        drop(self.out.write(prefix));
        drop(self.out.write_all(msg));
    }

    pub(crate) fn done(mut self) {
        self.done = true;
    }
}

impl<W: Write + ?Sized> Drop for Progress<'_, W> {
    fn drop(&mut self) {
        if !self.done && matches!(self.format, Format::Terminal) {
            drop(self.out.write(b"\n"));
        }
    }
}

pub(crate) fn prefix(format: Format) -> &'static str {
    match format {
        Format::Terminal => "\x1b[2K\r",
        Format::Newline | Format::No => "",
    }
}

pub(crate) fn report_line(
    format: Format,
    completed: usize,
    total: Option<usize>,
    msg: &str,
    out: &mut (impl Write + ?Sized),
) {
    struct Total(Option<usize>);
    impl std::fmt::Display for Total {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self.0 {
                Some(n) => write!(f, "{n}"),
                None => write!(f, "?"),
            }
        }
    }
    let total = Total(total);

    if msg.is_empty() {
        match format {
            Format::No => (),
            Format::Terminal => drop(write!(out, "\x1b[2K\r[{completed}/{total}]")),
            Format::Newline => drop(writeln!(out, "[{completed}/{total}]")),
        }
    } else {
        match format {
            Format::No => (),
            Format::Terminal => {
                let shorter = &msg[0..cmp::min(60, msg.len())];
                drop(write!(out, "\x1b[2K\r[{completed}/{total}] {shorter}"));
            }
            Format::Newline => {
                drop(writeln!(out, "[{completed}/{total}] {msg}"));
            }
        }
    }
    drop(out.flush());
}
