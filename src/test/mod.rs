mod warn;

use crate::{
    cache::{self, CacheWriter},
    cli, cmd, file, plan, run,
};

use anyhow::{Context, Result};
use clap::Parser as _;
use expect_test::expect;
use std::{
    num::NonZeroUsize,
    path::{Path, PathBuf},
    process,
};

#[derive(Debug)]
struct TestScenario {
    config: crate::config::Config,
    files: Vec<TestFile>,
    expected_output: Vec<String>,
    failed_commands: Vec<String>,
    run: Option<cli::Run>,
    color: cli::log::Color,
}

#[derive(Clone, Debug)]
struct TestFile {
    path: PathBuf,
    size: usize,
    content: String,
}

impl TestFile {
    fn to_file(&self) -> file::File {
        use xxhash_rust::xxh3::Xxh3;
        let mut metadata_hasher = Xxh3::new();
        metadata_hasher.update(self.path.as_os_str().as_encoded_bytes());
        metadata_hasher.update(&self.size.to_le_bytes());
        let metadata_stamp = file::Stamp(file::Xxhash(metadata_hasher.digest()));
        let mtime_stamp = file::Stamp(file::Xxhash(0));
        let content_stamp = Some(file::Stamp(file::compute_hash(self.content.as_bytes())));
        file::File {
            path: self.path.clone(),
            size: self.size,
            metadata_stamp,
            mtime_stamp,
            content_stamp,
        }
    }
}

fn process_config_section(scenario: &mut TestScenario, content: &str, path: &Path) -> Result<()> {
    scenario.config = toml::from_str(content.trim())
        .with_context(|| format!("Failed to parse config in test file: {}", path.display()))?;
    Ok(())
}

fn process_output_section(scenario: &mut TestScenario, content: &str) {
    // Parse output: batches are separated by blank lines
    let mut batches = Vec::new();
    let mut current_batch = Vec::new();
    for output_line in content.lines() {
        let trimmed = output_line.trim();
        if trimmed.is_empty() {
            // Blank line separates batches
            if !current_batch.is_empty() {
                batches.push(std::mem::take(&mut current_batch));
            }
        } else {
            current_batch.push(trimmed.to_string());
        }
    }
    // Add remaining batch if any
    if !current_batch.is_empty() {
        batches.push(current_batch);
    }
    // Flatten batches with blank lines between them
    for (i, batch) in batches.iter().enumerate() {
        if i > 0 {
            scenario.expected_output.push(String::new());
        }
        scenario.expected_output.append(&mut batch.clone());
    }
}

fn process_fail_section(scenario: &mut TestScenario, content: &str) {
    // Parse fail section: same format as output section
    let mut batches = Vec::new();
    let mut current_batch = Vec::new();
    for fail_line in content.lines() {
        let trimmed = fail_line.trim();
        if trimmed.is_empty() {
            // Blank line separates batches
            if !current_batch.is_empty() {
                batches.push(std::mem::take(&mut current_batch));
            }
        } else {
            current_batch.push(trimmed.to_string());
        }
    }
    // Add remaining batch if any
    if !current_batch.is_empty() {
        batches.push(current_batch);
    }
    // Flatten batches with blank lines between them
    for (i, batch) in batches.iter().enumerate() {
        if i > 0 {
            scenario.failed_commands.push(String::new());
        }
        scenario.failed_commands.append(&mut batch.clone());
    }
}

fn process_flags_section(scenario: &mut TestScenario, content: &str) {
    let mut args = vec!["lun".to_string()];
    for flag_line in content.lines() {
        let trimmed = flag_line.trim();
        if trimmed.is_empty() {
            continue;
        }
        for arg in trimmed.split_whitespace() {
            args.push(arg.to_string());
        }
    }

    let cli = cli::Cli::try_parse_from(args.iter().map(|s| s.as_str()))
        .map_err(|e| e.to_string())
        .unwrap();
    if let cli::Command::Run(run) = cli.command {
        scenario.run = Some(run);
    }
    scenario.color = cli.log.color;
}

fn process_files_section_line(scenario: &mut TestScenario, line: &str) {
    if line.starts_with("- `") && line.contains(":") {
        // Parse file entry like: - `file.py`: 100b
        let file_part = line
            .strip_prefix("- `")
            .and_then(|s| s.split("`: ").next())
            .unwrap_or("");
        let size_part = line
            .split(": ")
            .nth(1)
            .and_then(|s| s.strip_suffix('b'))
            .and_then(|s| s.trim().parse::<usize>().ok())
            .unwrap_or(0);

        // Generate content based on size
        let content = "x".repeat(size_part);
        scenario.files.push(TestFile {
            path: PathBuf::from(file_part),
            size: size_part,
            content,
        });
    }
}

fn process_section_content(
    scenario: &mut TestScenario,
    section: &str,
    content: &str,
    path: &Path,
) -> Result<()> {
    match section {
        "config" => process_config_section(scenario, content, path)?,
        "output" => process_output_section(scenario, content),
        "fail" => process_fail_section(scenario, content),
        "flags" => process_flags_section(scenario, content),
        _ => {}
    }
    Ok(())
}

// generated by AI ¯\_(ツ)_/¯
fn parse_test_file(path: &Path) -> Result<Vec<TestScenario>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read test file: {}", path.display()))?;

    let mut scenarios = Vec::new();
    let mut current_scenario: Option<TestScenario> = None;
    let mut current_section: Option<String> = None;
    let mut current_content = String::new();
    let mut previous_files: Vec<TestFile> = Vec::new();
    let mut in_code_block = false;

    for line in content.lines() {
        if line.starts_with("## Scenario ") {
            if let Some(scenario) = current_scenario.take() {
                // Save files for next scenario
                if !scenario.files.is_empty() {
                    previous_files = scenario.files.clone();
                }
                scenarios.push(scenario);
            }
            // Start new scenario, inherit files from previous if available
            let files = if previous_files.is_empty() {
                Vec::new()
            } else {
                previous_files.clone()
            };
            current_scenario = Some(TestScenario {
                config: crate::config::Config {
                    warns: crate::config::WarnCfg {
                        allow: Vec::new(),
                        warn: Vec::new(),
                        deny: Vec::new(),
                    },
                    linter: Vec::new(),
                    formatter: Vec::new(),
                    refs: Vec::new(),
                    careful: false,
                    cores: None,
                    mtime: false,
                    ninja: None,
                    ignore: Vec::new(),
                },
                files,
                expected_output: Vec::new(),
                failed_commands: Vec::new(),
                run: None,
                color: cli::log::Color::Auto,
            });
            current_section = None;
            current_content.clear();
            in_code_block = false;
        } else if line == "### Config" {
            current_section = Some("config".to_string());
            current_content.clear();
            in_code_block = false;
        } else if line == "### Files" {
            current_section = Some("files".to_string());
            current_content.clear();
            in_code_block = false;
            // Clear files when Files section starts (new files will be added)
            if let Some(ref mut scenario) = current_scenario {
                scenario.files.clear();
            }
        } else if line == "### Output" {
            current_section = Some("output".to_string());
            current_content.clear();
            in_code_block = false;
        } else if line == "### Fail" {
            current_section = Some("fail".to_string());
            current_content.clear();
            in_code_block = false;
        } else if line == "### Flags" {
            current_section = Some("flags".to_string());
            current_content.clear();
            in_code_block = false;
        } else if line.starts_with("```") {
            if let Some(ref section) = current_section {
                if line == "```toml" || line == "```sh" {
                    // Start of code block
                    current_content.clear();
                    in_code_block = true;
                } else if line == "```" {
                    if in_code_block {
                        // End of code block
                        if let Some(ref mut scenario) = current_scenario {
                            process_section_content(
                                scenario,
                                section.as_str(),
                                &current_content,
                                path,
                            )?;
                        }
                        current_content.clear();
                        in_code_block = false;
                    } else {
                        // Start of code block (plain ```)
                        current_content.clear();
                        in_code_block = true;
                    }
                }
            }
        } else if let Some(ref section) = current_section {
            if section == "files" {
                if let Some(ref mut scenario) = current_scenario {
                    process_files_section_line(scenario, line);
                }
            } else {
                current_content.push_str(line);
                current_content.push('\n');
            }
        }
    }

    if let Some(scenario) = current_scenario {
        scenarios.push(scenario);
    }

    Ok(scenarios)
}

fn command_to_string(cmd: &process::Command) -> String {
    let program = cmd.get_program().to_string_lossy();
    let args: Vec<String> = cmd
        .get_args()
        .map(|arg| arg.to_string_lossy().to_string())
        .collect();
    let cmd_str = format!("{} {}", program, args.join(" "));
    if let Some(cd_path) = cmd.get_current_dir() {
        format!("cd {} && {}", cd_path.display(), cmd_str)
    } else {
        cmd_str
    }
}

fn jobs_to_string(jobs: &[cmd::Command]) -> Vec<String> {
    let mut result = Vec::new();
    for job in jobs {
        let cmd = job.to_command();
        let cmd_str = command_to_string(&cmd);
        result.push(cmd_str);
    }
    result
}

fn test(path: &'static str) {
    let test_file = PathBuf::from(path);
    let scenarios = parse_test_file(&test_file).unwrap();
    assert!(!scenarios.is_empty());
    let mut cache = cache::HashCache::new(PathBuf::from(".lun"));
    for (i, scenario) in scenarios.iter().enumerate() {
        let default = cli::Run::try_parse_from(["run"])
            .map_err(|e| e.to_string())
            .unwrap();
        let run = scenario.run.as_ref().unwrap_or(&default);

        let mut files = scenario
            .files
            .iter()
            .map(TestFile::to_file)
            .collect::<Vec<_>>();
        run::filter_files(&mut files, &run.only_files, &run.skip_files).unwrap();
        let cores = scenario
            .config
            .cores
            .unwrap_or(const { NonZeroUsize::new(1).unwrap() });
        let run_mode = run::RunMode::from(run);
        let tool =
            scenario
                .config
                .linter
                .iter()
                .cloned()
                .map(|t| t.into_tool(run_mode, false, scenario.color, &scenario.config.ignore))
                .chain(
                    scenario.config.formatter.iter().cloned().map(|t| {
                        t.into_tool(run_mode, false, scenario.color, &scenario.config.ignore)
                    }),
                )
                .collect::<Result<Vec<_>>>()
                .unwrap();
        let batches =
            plan::plan(&mut cache, &tool, &files, &[], cores, run.no_batch, false).unwrap();
        let out = jobs_to_string(&batches);
        assert_eq!(
            out,
            scenario.expected_output,
            "Scenario {} output mismatch",
            i + 1
        );
        // Simulate executing batches by marking commands as done in the cache
        // Skip commands that are in the failed_commands list
        let failed_set: std::collections::HashSet<String> = scenario
            .failed_commands
            .iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect();
        for cmd in &batches {
            let cmd_str = command_to_string(&cmd.to_command());
            // Only mark as done if this command is not in the failed list
            if !failed_set.contains(&cmd_str) {
                let tool = cmd.tool.clone();
                for file in &cmd.files {
                    let key = cache::Key::new(file.content_stamp(), tool.stamp);
                    cache.done(&key);
                }
            }
        }
    }
}

#[test]
fn parse_test_file_debug() {
    let test_file = PathBuf::from("tests/changing-cli.md");
    let scenarios = parse_test_file(&test_file).unwrap();
    let debug_output = format!("{scenarios:#?}");
    expect![[r#"
        [
            TestScenario {
                config: Config {
                    linter: [
                        Linter {
                            tool: Tool {
                                name: None,
                                cmd: "lint --",
                                files: [
                                    "*.py",
                                ],
                                ignore: [],
                                granularity: Individual,
                                configs: [],
                                cd: None,
                            },
                            fix: None,
                        },
                    ],
                    formatter: [],
                    careful: false,
                    cores: None,
                    ignore: [],
                    mtime: false,
                    ninja: None,
                    refs: [],
                    warns: WarnCfg {
                        allow: [],
                        warn: [],
                        deny: [],
                    },
                },
                files: [
                    TestFile {
                        path: "file.py",
                        size: 8,
                        content: "xxxxxxxx",
                    },
                ],
                expected_output: [
                    "lint -- file.py",
                ],
                failed_commands: [],
                run: None,
                color: Auto,
            },
            TestScenario {
                config: Config {
                    linter: [
                        Linter {
                            tool: Tool {
                                name: None,
                                cmd: "lint --some-flag --",
                                files: [
                                    "*.py",
                                ],
                                ignore: [],
                                granularity: Individual,
                                configs: [],
                                cd: None,
                            },
                            fix: None,
                        },
                    ],
                    formatter: [],
                    careful: false,
                    cores: None,
                    ignore: [],
                    mtime: false,
                    ninja: None,
                    refs: [],
                    warns: WarnCfg {
                        allow: [],
                        warn: [],
                        deny: [],
                    },
                },
                files: [
                    TestFile {
                        path: "file.py",
                        size: 8,
                        content: "xxxxxxxx",
                    },
                ],
                expected_output: [
                    "lint --some-flag -- file.py",
                ],
                failed_commands: [],
                run: None,
                color: Auto,
            },
        ]"#]]
    .assert_eq(&debug_output);
}

#[test]
fn batch2() {
    test("tests/batch2.md");
}

#[test]
fn batch3() {
    test("tests/batch3.md");
}

#[test]
fn cd() {
    test("tests/cd.md");
}

#[test]
fn changing_cli() {
    test("tests/changing-cli.md");
}

#[test]
fn color() {
    test("tests/color.md");
}

#[test]
fn fail() {
    test("tests/fail.md");
}

#[test]
fn format() {
    test("tests/format.md");
}

#[test]
fn no_batch() {
    test("tests/no-batch.md");
}

#[test]
fn only_files() {
    test("tests/only-files.md");
}

#[test]
fn skip_files() {
    test("tests/skip-files.md");
}

#[test]
fn twice() {
    test("tests/twice.md");
}
