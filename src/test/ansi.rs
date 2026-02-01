use std::path::PathBuf;

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
