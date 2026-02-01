use std::{fs, path::PathBuf};

fn test_file(name: &str) {
    let path = PathBuf::from(format!("tests/ansi/{name}.md"));
    super::e2e::run_test(&path, true).unwrap();
}

#[test]
fn ux_success() {
    test_file("ux-success");
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
fn ux_err_multiple_failures() {
    test_file("ux-err-multiple-failures");
}

#[test]
fn all_ansi_files_have_tests() {
    // Read the ansi.rs source to find all test_file("...") calls
    let source = fs::read_to_string("src/test/ansi.rs").expect("Failed to read ansi.rs");
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

    // Read all .md files in tests/ansi/
    let mut files_on_disk: Vec<String> = fs::read_dir("tests/ansi")
        .expect("Failed to read tests/ansi directory")
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
            "ansi test coverage mismatch!\n\
             Files without tests: {missing_tests:?}\n\
             Tests without files: {extra_tests:?}",
        );
    }
}
