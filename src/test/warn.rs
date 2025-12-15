use std::path::PathBuf;

use clap::Parser as _;
use expect_test::expect;

fn test(flags: &[&'static str], config: &'static str) -> Result<(), anyhow::Error> {
    let cli = crate::cli::Cli::try_parse_from(std::iter::once("lun").chain(flags.iter().copied()))
        .map_err(|e| e.to_string())
        .unwrap();
    let mut cli = cli;
    cli.config = PathBuf::from("test.toml");
    let config = toml::from_str(config).unwrap();
    crate::go(cli, config).map(|b| assert!(b))
}

#[test]
fn unknown_tool_success() {
    test(
        &["run", "--dry-run", "--only-tool", "mylinter"],
        r#"
[[tool]]
name = "mylinter"
cmd = "lint --"
files = ["*.py"]
granularity = "individual"
"#,
    )
    .unwrap();
}

#[test]
fn unknown_tool_failure() {
    let result = test(
        &[
            "--deny=unknown-tool",
            "run",
            "--dry-run",
            "--only-tool=bogus",
        ],
        r#"
[[tool]]
name = "mylinter"
cmd = "lint --"
files = ["*.py"]
granularity = "individual"
"#,
    );
    let error_display = format!("{:#}", result.unwrap_err());
    expect!["found unknown tool names and --deny=unknown-tool"].assert_eq(&error_display);
}

#[test]
fn careful_success() {
    test(
        &["run", "--dry-run", "--careful"],
        r#"
[[tool]]
cmd = "lint --"
files = ["*.py"]
granularity = "individual"
"#,
    )
    .unwrap();
}

#[test]
fn careful_failure() {
    let result = test(
        &["--deny=careful", "run", "--dry-run"],
        r#"
[[tool]]
cmd = "lint --"
files = ["*.py"]
granularity = "individual"
"#,
    );
    let error_display = format!("{:#}", result.unwrap_err());
    expect!["--careful is not set and --deny=careful"].assert_eq(&error_display);
}

#[test]
fn mtime_success() {
    test(
        &["run", "--dry-run"],
        r#"
[[tool]]
cmd = "lint --"
files = ["*.py"]
granularity = "individual"
"#,
    )
    .unwrap();
}

#[test]
fn mtime_failure() {
    let result = test(
        &["--deny=mtime", "run", "--dry-run", "--mtime"],
        r#"
[[tool]]
cmd = "lint --"
files = ["*.py"]
granularity = "individual"
"#,
    );
    let error_display = format!("{:#}", result.unwrap_err());
    expect!["mtime is set and --deny=mtime"].assert_eq(&error_display);
}

#[test]
fn refs_success() {
    test(
        &["run", "--dry-run"],
        r#"
[[tool]]
cmd = "lint --"
files = ["*.py"]
granularity = "individual"
"#,
    )
    .unwrap();
}

#[test]
fn refs_failure() {
    let result = test(
        &["--deny=refs", "run", "--dry-run", "--refs", "main"],
        r#"
[[tool]]
cmd = "lint --"
files = ["*.py"]
granularity = "individual"
"#,
    );
    let error_display = format!("{:#}", result.unwrap_err());
    expect!["refs is used and --deny=refs"].assert_eq(&error_display);
}

#[test]
fn unknown_warn_success() {
    test(
        &["--deny=unknown-tool", "run", "--dry-run"],
        r#"
[[tool]]
cmd = "lint --"
files = ["*.py"]
granularity = "individual"
"#,
    )
    .unwrap();
}

#[test]
fn unknown_warn_failure() {
    let result = test(
        &["--allow=bogus-warn", "run", "--dry-run"],
        r#"
[[tool]]
cmd = "lint --"
files = ["*.py"]
granularity = "individual"
"#,
    );
    let error_display = format!("{:#}", result.unwrap_err());
    expect!["found unknown warning names and --deny=unknown-lint"].assert_eq(&error_display);
}

#[test]
fn no_files_success() {
    test(
        &["--allow=no-files", "run", "--dry-run"],
        r#"
[[tool]]
cmd = "lint --"
files = []
granularity = "individual"
"#,
    )
    .unwrap();
}

#[test]
fn no_files_failure() {
    let result = test(
        &["run", "--dry-run"],
        r#"
[[tool]]
cmd = "lint --"
files = []
granularity = "individual"
"#,
    );
    let error_display = format!("{:#}", result.unwrap_err());
    expect!["found tools with empty `files` arrays and --deny=no-files"].assert_eq(&error_display);
}
