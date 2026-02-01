use std::io::Write;
use std::time::{Duration, Instant};

use crate::out;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Format {
    No,
    Terminal,
    Newline,
}

impl Format {
    pub(crate) fn new(out_config: out::Config) -> Self {
        if out_config.verbosity < tracing::Level::INFO {
            return Self::No;
        }
        if out_config.interactive {
            Self::Terminal
        } else {
            Self::Newline
        }
    }
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
            report_line(self.format, self.completed, self.total, msg, self.out, true);
        }
    }

    /// Report progress at completed+1 (showing the item being worked on).
    pub(crate) fn report(&mut self, msg: &str) {
        if self.should_write() {
            report_line(
                self.format,
                self.completed + 1,
                self.total,
                msg,
                self.out,
                true,
            );
        }
    }

    pub(crate) fn fail(&mut self, msg: &[u8]) {
        let prefix = match self.format {
            Format::Terminal => b"\n".as_slice(),
            Format::Newline | Format::No => b"".as_slice(),
        };
        drop(self.out.write(prefix));
        drop(self.out.write_all(b"FAILED:\n"));
        drop(self.out.write_all(msg));
    }

    pub(crate) fn done(mut self) {
        self.done = true;
    }

    pub(crate) fn finalize(self, msg: &str) {
        report_line(
            self.format,
            self.completed,
            self.total,
            msg,
            self.out,
            false,
        );
    }
}

impl<W: Write + ?Sized> Drop for Progress<'_, W> {
    fn drop(&mut self) {
        if !self.done && matches!(self.format, Format::Terminal) {
            drop(self.out.write(b"\n"));
        }
    }
}

pub(crate) fn report_line(
    format: Format,
    completed: usize,
    total: Option<usize>,
    msg: &str,
    out: &mut (impl Write + ?Sized),
    trunc: bool,
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
                if msg.len() > 60 && trunc {
                    let shorter = &msg[0..60];
                    drop(write!(out, "\x1b[2K\r[{completed}/{total}] {shorter}..."));
                } else {
                    drop(write!(out, "\x1b[2K\r[{completed}/{total}] {msg}"));
                }
            }
            Format::Newline => {
                drop(writeln!(out, "[{completed}/{total}] {msg}"));
            }
        }
    }
    drop(out.flush());
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::expect;

    fn to_str(buf: &[u8]) -> &str {
        std::str::from_utf8(buf).unwrap()
    }

    #[test]
    fn report_line_terminal_empty_msg() {
        let mut buf = Vec::new();
        report_line(Format::Terminal, 5, Some(10), "", &mut buf, true);
        expect![[r#"\u{1b}[2K\r[5/10]"#]].assert_eq(&to_str(&buf).escape_default().to_string());
    }

    #[test]
    fn report_line_terminal_truncates_long_msg() {
        let mut buf = Vec::new();
        let long_msg = "a".repeat(100);
        report_line(Format::Terminal, 1, Some(5), &long_msg, &mut buf, true);
        expect![[
            r#"\u{1b}[2K\r[1/5] aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa..."#
        ]]
        .assert_eq(&to_str(&buf).escape_default().to_string());
    }

    #[test]
    fn report_line_newline_with_msg() {
        let mut buf = Vec::new();
        report_line(Format::Newline, 3, Some(10), "working", &mut buf, true);
        expect![[r#"
            [3/10] working
        "#]]
        .assert_eq(to_str(&buf));
    }

    #[test]
    fn report_line_newline_empty_msg() {
        let mut buf = Vec::new();
        report_line(Format::Newline, 3, Some(10), "", &mut buf, true);
        expect![[r#"
            [3/10]
        "#]]
        .assert_eq(to_str(&buf));
    }

    #[test]
    fn report_line_newline_no_truncation() {
        let mut buf = Vec::new();
        let long_msg = "a".repeat(100);
        report_line(Format::Newline, 1, Some(5), &long_msg, &mut buf, true);
        expect![[r#"
            [1/5] aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
        "#]]
        .assert_eq(to_str(&buf));
    }

    #[test]
    fn report_line_no_format() {
        let mut buf = Vec::new();
        report_line(Format::No, 1, Some(10), "hello", &mut buf, false);
        expect![[""]].assert_eq(to_str(&buf));
    }

    #[test]
    fn progress_write_at_completed() {
        let mut buf = Vec::new();
        let mut progress = Progress::new(Format::Newline, Some(10), &mut buf);
        progress.completed = 5;
        progress.write("msg");
        progress.done();
        expect![[r#"
            [5/10] msg
        "#]]
        .assert_eq(to_str(&buf));
    }

    #[test]
    fn progress_fail_terminal_adds_newline() {
        let mut buf = Vec::new();
        let mut progress = Progress::new(Format::Terminal, Some(10), &mut buf);
        progress.fail(b"error");
        progress.done();
        expect![[r#"\nFAILED:\nerror"#]].assert_eq(&to_str(&buf).escape_default().to_string());
    }

    #[test]
    fn progress_fail_newline_no_prefix() {
        let mut buf = Vec::new();
        let mut progress = Progress::new(Format::Newline, Some(10), &mut buf);
        progress.fail(b"error");
        progress.done();
        expect![[r#"
            FAILED:
            error"#]]
        .assert_eq(to_str(&buf));
    }

    #[test]
    fn progress_drop_terminal_adds_newline() {
        let mut buf = Vec::new();
        {
            let _progress = Progress::new(Format::Terminal, Some(10), &mut buf);
            // dropped without calling done()
        }
        expect![[r#"\n"#]].assert_eq(&to_str(&buf).escape_default().to_string());
    }

    #[test]
    fn progress_done_prevents_drop_newline() {
        let mut buf = Vec::new();
        {
            let progress = Progress::new(Format::Terminal, Some(10), &mut buf);
            progress.done();
        }
        expect![[""]].assert_eq(to_str(&buf));
    }

    #[test]
    fn progress_rate_limit_terminal() {
        let mut buf = Vec::new();
        let mut progress = Progress::new(Format::Terminal, Some(10), &mut buf);
        progress.write("first");
        progress.write("second"); // should be rate-limited
        progress.write("third"); // should be rate-limited
        progress.done();
        expect![[r#"\u{1b}[2K\r[0/10] first"#]]
            .assert_eq(&to_str(&buf).escape_default().to_string());
    }

    #[test]
    fn progress_no_rate_limit_newline() {
        let mut buf = Vec::new();
        let mut progress = Progress::new(Format::Newline, Some(10), &mut buf);
        progress.write("first");
        progress.write("second");
        progress.write("third");
        progress.done();
        expect![[r#"
            [0/10] first
            [0/10] second
            [0/10] third
        "#]]
        .assert_eq(to_str(&buf));
    }
}
