use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser as _;
use expect_test::expect;

use std::env;

use crate::out;

#[derive(Debug)]
struct Command {
    args: Vec<String>,
    expected_output: String,
    /// Line number where expected output starts (for `UPDATE_EXPECT`)
    expected_output_line: usize,
}

#[derive(Debug)]
struct TestCase {
    files: Vec<(PathBuf, String)>,
    setup_commands: Vec<String>,
    commands: Vec<Command>,
}

fn parse_test_file(path: &Path) -> Result<TestCase> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read test file: {}", path.display()))?;

    let mut files: Vec<(PathBuf, String)> = Vec::new();
    let mut setup_commands: Vec<String> = Vec::new();
    let mut commands: Vec<Command> = Vec::new();

    // Current command being built
    let mut current_command_args: Vec<String> = Vec::new();
    let mut current_command_output_line = 0;

    let mut current_section: Option<&str> = None;
    let mut current_file_path: Option<PathBuf> = None;
    let mut in_code_block = false;
    let mut code_block_content = String::new();
    let mut seen_command_args = false;

    for (line_num, line) in content.lines().enumerate() {
        let line_number = line_num + 1; // 1-indexed

        if line.starts_with("```") {
            if in_code_block {
                // End of code block
                match current_section {
                    Some("files") => {
                        if let Some(file_path) = current_file_path.take() {
                            files.push((file_path, code_block_content.clone()));
                        }
                    }
                    Some("setup") => {
                        // Each line in setup is a separate command
                        for cmd in code_block_content.lines() {
                            if !cmd.trim().is_empty() {
                                setup_commands.push(cmd.to_string());
                            }
                        }
                    }
                    Some("command") => {
                        #[allow(clippy::if_not_else)]
                        if !seen_command_args {
                            // First code block is the command args
                            current_command_args = code_block_content
                                .split_whitespace()
                                .map(String::from)
                                .collect();
                            seen_command_args = true;
                        } else {
                            // Second code block is expected output
                            // Save the completed command
                            commands.push(Command {
                                args: current_command_args.clone(),
                                expected_output: code_block_content.clone(),
                                expected_output_line: current_command_output_line,
                            });
                            current_command_args.clear();
                            seen_command_args = false;
                        }
                    }
                    _ => {}
                }
                in_code_block = false;
                code_block_content.clear();
            } else {
                // Start of code block
                in_code_block = true;
                // Record line number for expected output (the line after ```)
                if current_section == Some("command") && seen_command_args {
                    current_command_output_line = line_number + 1;
                }
            }
        } else if in_code_block {
            if !code_block_content.is_empty() {
                code_block_content.push('\n');
            }
            code_block_content.push_str(line);
        } else if line.starts_with("## Files") {
            current_section = Some("files");
        } else if line.starts_with("## Setup") {
            current_section = Some("setup");
        } else if line.starts_with("## Command") {
            current_section = Some("command");
            seen_command_args = false;
        } else if line.starts_with("### `") && current_section == Some("files") {
            // Parse file path from ### `filename`
            if let Some(end) = line[5..].find('`') {
                let file_path = &line[5..5 + end];
                current_file_path = Some(PathBuf::from(file_path));
            }
        }
    }

    Ok(TestCase {
        files,
        setup_commands,
        commands,
    })
}

fn update_expected_output(path: &Path, line_start: usize, new_output: &str) -> Result<()> {
    let content = fs::read_to_string(path)?;
    let lines: Vec<&str> = content.lines().collect();

    // Find the code block boundaries
    let mut start_line = None;
    let mut end_line = None;
    let mut in_target_block = false;

    for (i, line) in lines.iter().enumerate() {
        let line_num = i + 1;
        if line_num == line_start - 1 && line.starts_with("```") {
            in_target_block = true;
            start_line = Some(i);
        } else if in_target_block && line.starts_with("```") {
            end_line = Some(i);
            break;
        }
    }

    if let (Some(start), Some(end)) = (start_line, end_line) {
        let mut new_lines: Vec<&str> = lines[..=start].to_vec();
        for output_line in new_output.lines() {
            new_lines.push(output_line);
        }
        new_lines.extend_from_slice(&lines[end..]);

        let new_content = new_lines.join("\n");
        fs::write(path, new_content)?;
    }

    Ok(())
}

/// Run a single e2e test
fn run_test(test_path: &Path) -> Result<()> {
    let test_case = parse_test_file(test_path)?;

    let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
    let temp_path = temp_dir.path();

    // Create files
    for (file_path, contents) in &test_case.files {
        let full_path = temp_path.join(file_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&full_path, contents)?;
    }

    // Run setup commands
    for setup_cmd in &test_case.setup_commands {
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(setup_cmd)
            .current_dir(temp_path)
            .output()
            .with_context(|| format!("Failed to execute setup command: {setup_cmd}"))?;

        if !output.status.success() {
            anyhow::bail!(
                "Setup command failed: {}\nstderr: {}",
                setup_cmd,
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    let config_path = temp_path.join("lun.toml");
    let temp_path_str = temp_path.to_string_lossy();
    let env = crate::env::Env {
        cwd: temp_path.to_path_buf(),
        start_time: None,
        term_width: None,
    };
    let paths = crate::Paths {
        config: Some(config_path.clone()),
        cache: temp_path.join(".lun"),
    };

    // Run each command
    for command in &test_case.commands {
        let mut cli_args = vec!["lun".to_string()];
        cli_args.push("--config".to_string());
        cli_args.push(temp_path.join("lun.toml").to_string_lossy().to_string());
        cli_args.push("--cache".to_string());
        cli_args.push(temp_path.join(".lun").to_string_lossy().to_string());
        cli_args.extend(command.args.clone());
        if command.args.starts_with(&["run".to_string()]) {
            cli_args.push("--jobs".to_string());
            cli_args.push("1".to_string());
        }
        let cli = crate::cli::Cli::try_parse_from(&cli_args)
            .map_err(|e| anyhow::anyhow!("Failed to parse CLI: {e}"))?;

        let config = match crate::config::Config::load(&config_path) {
            Ok(c) => c,
            Err(e) => {
                // Config loading failed - treat this as the error output
                let actual_output = format!("{e}").replace(temp_path_str.as_ref(), "<TEMP>");
                let expected_normalized = &command.expected_output;
                let actual_normalized = actual_output.trim();

                if env::var("UPDATE_EXPECT").is_ok() {
                    if expected_normalized != actual_normalized {
                        update_expected_output(
                            test_path,
                            command.expected_output_line,
                            &actual_output,
                        )?;
                        println!("Updated expected output in {}", test_path.display());
                    }
                    continue;
                }

                if expected_normalized != actual_normalized {
                    anyhow::bail!(
                        "Output mismatch in {}:\n\nExpected:\n{}\n\nActual:\n{}\n",
                        test_path.display(),
                        expected_normalized,
                        actual_normalized
                    );
                }
                continue;
            }
        };

        // Capture output
        let mut output_buffer: Vec<u8> = Vec::new();

        // Run with the temp directory as working directory
        let out_config = out::Config::new(&env, cli.log);
        let result = crate::go(
            cli.command,
            cli.warn,
            &paths,
            &env,
            config,
            out_config,
            &mut output_buffer,
        );

        // Get captured output and normalize escape sequences
        let captured = {
            let raw = String::from_utf8_lossy(&output_buffer).to_string();
            // Replace terminal clear-line escape sequences with newlines for readability
            raw.replace("\x1b[2K\r", "")
        };

        // Handle errors by capturing them as output
        let actual_output = match result {
            Ok(_) => captured,
            Err(e) => format!("{captured}{e}").trim().to_string(),
        };

        // Normalize outputs for comparison (trim whitespace, normalize line endings, replace temp paths)
        let expected_normalized = &command.expected_output;
        let actual_normalized = actual_output
            .trim()
            .replace(temp_path_str.as_ref(), "<TEMP>");
        let actual_normalized = actual_normalized.trim();

        // Check UPDATE_EXPECT
        if env::var("UPDATE_EXPECT").is_ok() {
            if expected_normalized != actual_normalized {
                let output_for_file = actual_output
                    .trim()
                    .replace(temp_path_str.as_ref(), "<TEMP>");
                update_expected_output(test_path, command.expected_output_line, &output_for_file)?;
                println!("Updated expected output in {}", test_path.display());
            }
            continue;
        }

        // Compare
        if expected_normalized != actual_normalized {
            anyhow::bail!(
                "Output mismatch in {}:\n\nExpected:\n{}\n\nActual:\n{}\n",
                test_path.display(),
                expected_normalized,
                actual_normalized
            );
        }
    }

    Ok(())
}

#[test]
fn parse_test_file_debug() {
    let test_file = PathBuf::from("tests/e2e/ux-success.md");
    let test_case = parse_test_file(&test_file).unwrap();
    let debug_output = format!("{test_case:#?}");
    expect![[r#"
        TestCase {
            files: [
                (
                    "lun.toml",
                    "[[linter]]\nname = \"echo\"\ncmd = \"echo\"\nfiles = [\"*.toml\"]",
                ),
            ],
            setup_commands: [],
            commands: [
                Command {
                    args: [
                        "run",
                    ],
                    expected_output: "[0/?] Collecting files\n[1/1] Planning\n[1/1] echo (1/1)\n[1/1] 1 file linted",
                    expected_output_line: 23,
                },
            ],
        }"#]]
    .assert_eq(&debug_output);
}

fn test_file(name: &str) {
    let path = PathBuf::from(format!("tests/e2e/{name}.md"));
    run_test(&path).unwrap();
}

#[test]
fn args_none_no_needed_files() {
    test_file("args-none-no-needed-files");
}

#[test]
fn gitignore() {
    test_file("gitignore");
}

#[test]
fn global_ignore() {
    test_file("global-ignore");
}

#[test]
fn ux_err_bogus_command() {
    test_file("ux-err-bogus-command");
}

#[test]
fn ux_err_invalid_glob() {
    test_file("ux-err-invalid-glob");
}

#[test]
fn ux_err_invalid_toml() {
    test_file("ux-err-invalid-toml");
}

#[test]
fn ux_err_missing_config() {
    test_file("ux-err-missing-config");
}

#[test]
fn ux_err_unknown_tool_config() {
    test_file("ux-err-unknown-tool-config");
}

#[test]
fn ux_err_deny_unknown_tool() {
    test_file("ux-err-deny-unknown-tool");
}

#[test]
fn ux_err_multiple_failures() {
    test_file("ux-err-multiple-failures");
}

#[test]
fn ux_failure() {
    test_file("ux-failure");
}

#[test]
fn ux_failure_then_success() {
    test_file("ux-failure-then-success");
}

#[test]
fn ux_known() {
    test_file("ux-known");
}

#[test]
fn ux_long_path_name() {
    test_file("ux-long-path");
}

#[test]
fn ux_noop() {
    test_file("ux-noop");
}

#[test]
fn ux_success() {
    test_file("ux-success");
}

#[test]
fn success_then_failure() {
    test_file("ux-success-then-failure");
}

#[test]
fn ux_quiet() {
    test_file("ux-quiet");
}

#[test]
fn vcs() {
    test_file("vcs");
}

#[test]
fn all_e2e_files_have_tests() {
    // Read the e2e.rs source to find all test_file("...") calls
    let source = fs::read_to_string("src/test/e2e.rs").expect("Failed to read e2e.rs");
    let mut tested: Vec<String> = source
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("test_file(\"") {
                let start = trimmed.find('"')? + 1;
                let end = trimmed[start..].find('"')? + start;
                Some(trimmed[start..end].to_string())
            } else {
                None
            }
        })
        .collect();
    tested.sort();

    // Read all .md files in tests/e2e/
    let mut files_on_disk: Vec<String> = fs::read_dir("tests/e2e")
        .expect("Failed to read tests/e2e directory")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension()? == "md" {
                path.file_stem()?.to_str().map(String::from)
            } else {
                None
            }
        })
        .collect();
    files_on_disk.sort();

    let missing_tests: Vec<_> = files_on_disk
        .iter()
        .filter(|f| !tested.contains(f))
        .collect();

    let extra_tests: Vec<_> = tested
        .iter()
        .filter(|f| !files_on_disk.contains(f))
        .collect();

    if !missing_tests.is_empty() || !extra_tests.is_empty() {
        panic!(
            "e2e test coverage mismatch!\n\
             Files without tests: {missing_tests:?}\n\
             Tests without files: {extra_tests:?}",
        );
    }
}
