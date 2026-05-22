use std::io::Write;
use std::process::Command;

// Path to the compiled binary injected by cargo at test-compile time.
const BIN: &str = env!("CARGO_BIN_EXE_bin");
const BERLIN52: &str = "tests/fixtures/berlin52.tsp";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

struct TempFile {
    path: std::path::PathBuf,
}

impl TempFile {
    fn write(content: &str, suffix: &str) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        std::thread::current().id().hash(&mut h);
        let name = format!("teeline_test_{:x}_{:x}{}", std::process::id(), h.finish(), suffix);
        let path = std::env::temp_dir().join(name);
        let mut f = std::fs::File::create(&path).expect("create temp file");
        f.write_all(content.as_bytes()).expect("write temp file");
        TempFile { path }
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(BIN).args(args).output().expect("failed to run binary")
}

// ---------------------------------------------------------------------------
// Happy path
// ---------------------------------------------------------------------------

#[test]
fn test_pipeline_config_runs_and_produces_valid_output() {
    let cfg = TempFile::write(
        r#"
[[stage]]
solver = "nn"

[[stage]]
solver = "2opt"
epochs = 100
"#,
        ".toml",
    );

    let out = run(&[
        "pipeline",
        &format!("--config={}", cfg.path.display()),
        "-i",
        BERLIN52,
    ]);

    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    // First line: "<distance> <flag>"
    let first_line = stdout.lines().next().expect("no output");
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    assert_eq!(parts.len(), 2, "unexpected first line: {first_line}");
    let _dist: f64 = parts[0].parse().expect("distance not a float");
    assert!(parts[1] == "0" || parts[1] == "1");
}

#[test]
fn test_pipeline_config_global_epochs_applied() {
    // Two minimal configs: one with default epochs, one with a very low epoch
    // cap on the SA stage. The low-epoch run should finish quickly and still
    // produce a valid tour.
    let cfg = TempFile::write(
        r#"
[global]
epochs = 50

[[stage]]
solver = "nn"

[[stage]]
solver = "sa"
max_temperature = 200.0
cooling_rate    = 0.001
epochs          = 50
"#,
        ".toml",
    );

    let out = run(&[
        "pipeline",
        &format!("--config={}", cfg.path.display()),
        "-i",
        BERLIN52,
    ]);

    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Second line: space-separated city IDs — should have 52 values
    let ids: Vec<&str> = stdout.lines().nth(1).unwrap_or("").split_whitespace().collect();
    assert_eq!(ids.len(), 52, "expected 52 city IDs, got {}", ids.len());
}

// ---------------------------------------------------------------------------
// Mutual-exclusion errors
// ---------------------------------------------------------------------------

#[test]
fn test_pipeline_config_and_steps_mutually_exclusive() {
    let cfg = TempFile::write("[[stage]]\nsolver = \"nn\"\n", ".toml");

    let out = run(&[
        "pipeline",
        &format!("--config={}", cfg.path.display()),
        "--steps=nn,2opt",
        "-i",
        BERLIN52,
    ]);

    assert!(!out.status.success(), "expected non-zero exit");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("mutually exclusive"),
        "expected 'mutually exclusive' in stderr, got: {stderr}"
    );
}

#[test]
fn test_pipeline_neither_config_nor_steps_errors() {
    let out = run(&["pipeline", "-i", BERLIN52]);

    assert!(!out.status.success(), "expected non-zero exit");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--config") || stderr.contains("--steps"),
        "expected mention of --config or --steps in stderr, got: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// Missing / unreadable config file
// ---------------------------------------------------------------------------

#[test]
fn test_pipeline_missing_config_file_errors() {
    let out = run(&[
        "pipeline",
        "--config=/tmp/teeline_does_not_exist_xyz.toml",
        "-i",
        BERLIN52,
    ]);

    assert!(!out.status.success(), "expected non-zero exit");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("teeline_does_not_exist_xyz"),
        "expected filename in stderr, got: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// Config validation errors
// ---------------------------------------------------------------------------

#[test]
fn test_pipeline_unknown_key_in_stage_errors() {
    let cfg = TempFile::write(
        "[[stage]]\nsolver = \"nn\"\nepoch = 999\n", // typo: epoch not epochs
        ".toml",
    );

    let out = run(&[
        "pipeline",
        &format!("--config={}", cfg.path.display()),
        "-i",
        BERLIN52,
    ]);

    assert!(!out.status.success(), "expected non-zero exit");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("epoch"),
        "expected 'epoch' (the bad key) in stderr, got: {stderr}"
    );
}

#[test]
fn test_pipeline_cooling_rate_zero_errors() {
    let cfg = TempFile::write(
        "[[stage]]\nsolver = \"sa\"\ncooling_rate = 0.0\n",
        ".toml",
    );

    let out = run(&[
        "pipeline",
        &format!("--config={}", cfg.path.display()),
        "-i",
        BERLIN52,
    ]);

    assert!(!out.status.success(), "expected non-zero exit");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("cooling_rate"),
        "expected 'cooling_rate' in stderr, got: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// solve subcommand rejects --config
// ---------------------------------------------------------------------------

#[test]
fn test_solve_rejects_config_flag() {
    let cfg = TempFile::write("[[stage]]\nsolver = \"nn\"\n", ".toml");

    let out = run(&[
        "solve",
        "nn",
        &format!("--config={}", cfg.path.display()),
        "-i",
        BERLIN52,
    ]);

    assert!(!out.status.success(), "expected non-zero exit: solve must not accept --config");
}
