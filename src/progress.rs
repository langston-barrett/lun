use std::cmp;
use std::io::Write;
use std::time::{Duration, Instant};

use crate::out;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Format {
    No,
    Terminal(Option<u16>),
    Newline,
}

impl Format {
    pub(crate) fn new(out_config: out::Config, term_width: Option<u16>) -> Self {
        if out_config.verbosity < tracing::Level::INFO {
            return Self::No;
        }
        if out_config.interactive {
            Self::Terminal(term_width)
        } else {
            Self::Newline
        }
    }
}

const TERMINAL_RATE_LIMIT: Duration = Duration::from_millis(100);

pub(crate) struct Progress<'a, W: Write + ?Sized> {
    pub(crate) format: Format,
    pub(crate) completed: usize,
    pub(crate) total: Option<usize>,
    #[allow(dead_code)]
    interval: usize,
    color: bool,
    out: &'a mut W,
    done: bool,
    last_write: Option<Instant>,
}

impl<'a, W: Write + ?Sized> Progress<'a, W> {
    pub(crate) fn new(
        format: Format,
        total: Option<usize>,
        interval: Option<usize>,
        color: bool,
        out: &'a mut W,
    ) -> Self {
        Self {
            format,
            completed: 0,
            total,
            interval: cmp::max(interval.unwrap_or(1), 1),
            color,
            out,
            done: false,
            last_write: None,
        }
    }

    pub(crate) fn increment(&mut self) {
        self.completed += 1;
    }

    fn should_write(&mut self) -> bool {
        if !matches!(self.format, Format::Terminal(_)) {
            if self.interval != 1
                && let Some(t) = self.total
            {
                let interval = t / self.interval;
                let interval = if interval == 0 { 1 } else { interval };
                return self.completed == 0
                    || self.completed == t
                    || self.completed.is_multiple_of(interval);
            }
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
            report_line(
                self.format,
                self.completed,
                self.total,
                msg,
                self.out,
                true,
                false,
                false,
            );
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
                false,
                false,
            );
        }
    }

    pub(crate) fn fail(&mut self, msg: &[u8]) {
        let prefix = match self.format {
            Format::Terminal(_) => b"\n".as_slice(),
            Format::Newline | Format::No => b"".as_slice(),
        };
        drop(self.out.write(prefix));
        let failed_text = if self.color {
            "\x1b[31mFAILED\x1b[0m:\n"
        } else {
            "FAILED:\n"
        };
        drop(self.out.write_all(failed_text.as_bytes()));
        drop(self.out.write_all(msg));
    }

    pub(crate) fn done(mut self) {
        self.done = true;
    }

    pub(crate) fn finalize(self, msg: &str, errors: bool) {
        report_line(
            self.format,
            self.completed,
            self.total,
            msg,
            self.out,
            false,
            self.color,
            errors,
        );
    }
}

impl<W: Write + ?Sized> Drop for Progress<'_, W> {
    fn drop(&mut self) {
        if !self.done && matches!(self.format, Format::Terminal(_)) {
            drop(self.out.write(b"\n"));
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn report_line(
    format: Format,
    completed: usize,
    total: Option<usize>,
    msg: &str,
    out: &mut (impl Write + ?Sized),
    trunc: bool,
    color: bool,
    errors: bool,
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

    let progress = format!("[{completed}/{total}]");
    let progress = if color {
        if errors {
            format!("\x1b[31m{progress}\x1b[0m")
        } else {
            format!("\x1b[32m{progress}\x1b[0m")
        }
    } else {
        progress
    };

    if msg.is_empty() {
        match format {
            Format::No => (),
            Format::Terminal(_) => drop(write!(out, "\x1b[2K\r{progress}")),
            Format::Newline => drop(writeln!(out, "{progress}")),
        }
    } else {
        match format {
            Format::No => (),
            Format::Terminal(term_width) => {
                let progress_width = format!("[{completed}/{total}] ").len();
                let max_len = term_width.map(|w| {
                    usize::from(w).saturating_sub(progress_width.saturating_add("...".len()))
                });
                if let Some(max) = max_len
                    && msg.len() > max
                    && trunc
                {
                    let shorter = &msg[0..max];
                    drop(write!(out, "\x1b[2K\r{progress} {shorter}..."));
                } else {
                    drop(write!(out, "\x1b[2K\r{progress} {msg}"));
                }
            }
            Format::Newline => {
                drop(writeln!(out, "{progress} {msg}"));
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
        report_line(
            Format::Terminal(None),
            5,
            Some(10),
            "",
            &mut buf,
            true,
            false,
            false,
        );
        expect![[r#"\u{1b}[2K\r[5/10]"#]].assert_eq(&to_str(&buf).escape_default().to_string());
    }

    #[test]
    fn report_line_terminal_truncates_long_msg() {
        let mut buf = Vec::new();
        let long_msg = "a".repeat(100);
        // term_width of 80 means max_msg_len = 80 - 20 = 60
        report_line(
            Format::Terminal(Some(80)),
            1,
            Some(5),
            &long_msg,
            &mut buf,
            true,
            false,
            false,
        );
        expect![[r#"\u{1b}[2K\r[1/5] aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa..."#]]
        .assert_eq(&to_str(&buf).escape_default().to_string());
    }

    #[test]
    fn report_line_newline_with_msg() {
        let mut buf = Vec::new();
        report_line(
            Format::Newline,
            3,
            Some(10),
            "working",
            &mut buf,
            true,
            false,
            false,
        );
        expect![[r#"
            [3/10] working
        "#]]
        .assert_eq(to_str(&buf));
    }

    #[test]
    fn report_line_newline_empty_msg() {
        let mut buf = Vec::new();
        report_line(
            Format::Newline,
            3,
            Some(10),
            "",
            &mut buf,
            true,
            false,
            false,
        );
        expect![[r#"
            [3/10]
        "#]]
        .assert_eq(to_str(&buf));
    }

    #[test]
    fn report_line_newline_no_truncation() {
        let mut buf = Vec::new();
        let long_msg = "a".repeat(100);
        report_line(
            Format::Newline,
            1,
            Some(5),
            &long_msg,
            &mut buf,
            true,
            false,
            false,
        );
        expect![[r#"
            [1/5] aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
        "#]]
        .assert_eq(to_str(&buf));
    }

    #[test]
    fn report_line_no_format() {
        let mut buf = Vec::new();
        report_line(
            Format::No,
            1,
            Some(10),
            "hello",
            &mut buf,
            false,
            false,
            false,
        );
        expect![[""]].assert_eq(to_str(&buf));
    }

    #[test]
    fn progress_write_at_completed() {
        let mut buf = Vec::new();
        let mut progress = Progress::new(Format::Newline, Some(10), None, false, &mut buf);
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
        let mut progress = Progress::new(Format::Terminal(None), Some(10), None, false, &mut buf);
        progress.fail(b"error");
        progress.done();
        expect![[r#"\nFAILED:\nerror"#]].assert_eq(&to_str(&buf).escape_default().to_string());
    }

    #[test]
    fn progress_fail_newline_no_prefix() {
        let mut buf = Vec::new();
        let mut progress = Progress::new(Format::Newline, Some(10), None, false, &mut buf);
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
            let _progress = Progress::new(Format::Terminal(None), Some(10), None, false, &mut buf);
            // dropped without calling done()
        }
        expect![[r#"\n"#]].assert_eq(&to_str(&buf).escape_default().to_string());
    }

    #[test]
    fn progress_done_prevents_drop_newline() {
        let mut buf = Vec::new();
        {
            let progress = Progress::new(Format::Terminal(None), Some(10), None, false, &mut buf);
            progress.done();
        }
        expect![[""]].assert_eq(to_str(&buf));
    }

    #[test]
    fn progress_rate_limit_terminal() {
        let mut buf = Vec::new();
        let mut progress = Progress::new(Format::Terminal(None), Some(10), None, false, &mut buf);
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
        let mut progress = Progress::new(Format::Newline, Some(10), None, false, &mut buf);
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

    #[test]
    fn color_fail() {
        let mut buf = Vec::new();
        let mut progress = Progress::new(Format::Newline, Some(10), None, true, &mut buf);
        progress.fail(b"error");
        progress.done();
        expect![[r#"
            [31mFAILED[0m:
            error"#]]
        .assert_eq(to_str(&buf));
    }

    #[test]
    fn color_finalize_no_errors() {
        let mut buf = Vec::new();
        let progress = Progress::new(Format::Newline, Some(10), None, true, &mut buf);
        progress.finalize("done", false);
        expect![[r#"
            [32m[0/10][0m done
        "#]]
        .assert_eq(to_str(&buf));
    }

    #[test]
    fn color_finalize_with_errors() {
        let mut buf = Vec::new();
        let progress = Progress::new(Format::Newline, Some(10), None, true, &mut buf);
        progress.finalize("failed", true);
        expect![[r#"
            [31m[0/10][0m failed
        "#]]
        .assert_eq(to_str(&buf));
    }
}
