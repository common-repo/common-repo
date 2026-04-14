//! Shared test utilities for integration and E2E tests.
//!
//! This module provides common fixtures, helper functions, and macros
//! to reduce duplication across test files.
//!
//! ## Usage
//!
//! Add `mod common;` to your test file, then use the helpers:
//!
//! ```rust,ignore
//! mod common;
//! use common::prelude::*;
//!
//! #[test]
//! #[integration_test]
//! fn test_example() {
//!     let fixture = TestFixture::new().with_minimal_config();
//!     // ... test code
//! }
//! ```

use assert_fs::prelude::*;
use std::env;
use std::path::Path;
use std::process::Command;

/// Re-export commonly used test dependencies for convenience.
#[allow(unused_imports)]
pub mod prelude {
    pub use assert_cmd::cargo::cargo_bin_cmd;
    pub use assert_fs::prelude::*;
    pub use assert_fs::TempDir;
    pub use predicates::prelude::*;

    #[allow(unused_imports)]
    pub use super::configs;
    #[allow(unused_imports)]
    pub use super::init_test_git_repo;
    #[allow(unused_imports)]
    pub use super::should_skip_network_tests;
    #[allow(unused_imports)]
    pub use super::TestFixture;
}

/// Common configuration YAML snippets for testing.
#[allow(dead_code)]
pub mod configs {
    /// Minimal valid configuration with include patterns.
    pub const MINIMAL: &str = r#"
- include: ["**/*"]
"#;

    /// Minimal configuration with README only.
    pub const README_ONLY: &str = r#"
- include: ["README.md"]
"#;

    /// Configuration with include and exclude.
    pub const WITH_EXCLUDE: &str = r#"
- include: ["**/*"]
- exclude: ["target/**", ".git/**"]
"#;

    /// Invalid YAML for error testing.
    pub const INVALID_YAML: &str = "invalid: yaml: content:";

    /// Configuration referencing a repository.
    pub const WITH_REPO: &str = r#"
- repo:
    url: https://github.com/common-repo/common-repo.git
    ref: main
"#;

    /// Empty configuration (comments only).
    pub const EMPTY: &str = r#"# common-repo configuration
# Add your repository configurations here
[]
"#;
}

/// Check if network tests should be skipped.
///
/// Returns `true` if the `SKIP_NETWORK_TESTS` environment variable is set.
///
/// # Example
///
/// ```rust,ignore
/// if should_skip_network_tests() {
///     println!("Skipping network integration test");
///     return;
/// }
/// ```
#[allow(dead_code)]
pub fn should_skip_network_tests() -> bool {
    env::var("SKIP_NETWORK_TESTS").is_ok()
}

/// Initialize a local git repository for use as a test upstream.
///
/// Creates a git repo at the given path with the specified files,
/// optionally tags the initial commit.
///
/// Safety measures included:
/// - `commit.gpgsign false` to avoid GPG signing
/// - `core.hooksPath /dev/null` to disable pre-commit hooks
/// - `--no-verify` on commit to skip any remaining hooks
/// - Asserts that the commit succeeds
///
/// # Arguments
///
/// * `dir` - Temporary directory in which to create the repo
/// * `files` - Slice of `(path, content)` pairs to create and commit
/// * `tag` - Optional tag name to create on the initial commit
#[allow(dead_code)]
pub fn init_test_git_repo(
    dir: &assert_fs::TempDir,
    files: &[(&str, &str)],
    tag: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(dir.path())
        .output()?;
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(dir.path())
        .output()?;
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(dir.path())
        .output()?;
    Command::new("git")
        .args(["config", "commit.gpgsign", "false"])
        .current_dir(dir.path())
        .output()?;
    Command::new("git")
        .args(["config", "core.hooksPath", "/dev/null"])
        .current_dir(dir.path())
        .output()?;

    for (path, content) in files {
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(dir.path().join(parent))?;
            }
        }
        dir.child(path).write_str(content)?;
    }

    Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()?;
    let commit_output = Command::new("git")
        .args(["commit", "--no-verify", "-m", "Initial commit"])
        .current_dir(dir.path())
        .output()?;
    assert!(
        commit_output.status.success(),
        "git commit failed: {}",
        String::from_utf8_lossy(&commit_output.stderr)
    );

    if let Some(tag_name) = tag {
        Command::new("git")
            .args(["tag", tag_name])
            .current_dir(dir.path())
            .output()?;
    }

    Ok(())
}

/// A test fixture that provides a temporary directory with optional config.
///
/// This struct simplifies the common pattern of creating a temp directory
/// and populating it with a `.common-repo.yaml` configuration file.
///
/// # Example
///
/// ```rust,ignore
/// let fixture = TestFixture::new()
///     .with_config(configs::MINIMAL)
///     .with_file("test.txt", "hello world");
///
/// let mut cmd = cargo_bin_cmd!("common-repo");
/// cmd.current_dir(fixture.path())
///     .arg("ls")
///     .assert()
///     .success();
/// ```
pub struct TestFixture {
    temp_dir: assert_fs::TempDir,
}

impl TestFixture {
    /// Create a new test fixture with an empty temporary directory.
    pub fn new() -> Self {
        Self {
            temp_dir: assert_fs::TempDir::new().expect("Failed to create temp directory"),
        }
    }

    /// Add a `.common-repo.yaml` configuration file with the given content.
    pub fn with_config(self, content: &str) -> Self {
        self.temp_dir
            .child(".common-repo.yaml")
            .write_str(content)
            .expect("Failed to write config file");
        self
    }

    /// Add the minimal valid configuration.
    #[allow(dead_code)]
    pub fn with_minimal_config(self) -> Self {
        self.with_config(configs::MINIMAL)
    }

    /// Add a file with the given path and content.
    pub fn with_file(self, path: &str, content: &str) -> Self {
        self.temp_dir
            .child(path)
            .write_str(content)
            .expect("Failed to write file");
        self
    }

    /// Add a binary file with the given path and content.
    #[allow(dead_code)]
    pub fn with_binary_file(self, path: &str, content: &[u8]) -> Self {
        self.temp_dir
            .child(path)
            .write_binary(content)
            .expect("Failed to write binary file");
        self
    }

    /// Get the path to the temporary directory.
    pub fn path(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Get the path to the config file.
    pub fn config_path(&self) -> std::path::PathBuf {
        self.temp_dir.path().join(".common-repo.yaml")
    }

    /// Get access to the underlying TempDir for advanced usage.
    #[allow(dead_code)]
    pub fn temp_dir(&self) -> &assert_fs::TempDir {
        &self.temp_dir
    }

    /// Create a child path in the temp directory.
    #[allow(dead_code)]
    pub fn child(&self, path: &str) -> assert_fs::fixture::ChildPath {
        self.temp_dir.child(path)
    }
}

impl Default for TestFixture {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to create a CLI command for the common-repo binary.
///
/// This is a convenience wrapper around `cargo_bin_cmd!` that sets up
/// the command with the temp directory as the working directory.
///
/// # Example
///
/// ```rust,ignore
/// let fixture = TestFixture::new().with_minimal_config();
/// let mut cmd = fixture.command();
/// cmd.arg("ls").assert().success();
/// ```
impl TestFixture {
    /// Create a command configured to run in this fixture's directory.
    pub fn command(&self) -> assert_cmd::Command {
        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("common-repo");
        cmd.current_dir(self.path());
        cmd
    }

    /// Create a command with the config file path argument.
    #[allow(dead_code)]
    pub fn command_with_config(&self) -> assert_cmd::Command {
        let mut cmd = self.command();
        cmd.arg("--config").arg(self.config_path());
        cmd
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixture_creates_temp_dir() {
        let fixture = TestFixture::new();
        assert!(fixture.path().exists());
    }

    #[test]
    fn test_fixture_with_config() {
        let fixture = TestFixture::new().with_config("test: config");
        assert!(fixture.config_path().exists());
    }

    #[test]
    fn test_fixture_with_file() {
        let fixture = TestFixture::new().with_file("test.txt", "hello");
        assert!(fixture.path().join("test.txt").exists());
    }

    #[test]
    fn test_configs_are_valid_yaml() {
        // Verify that our config constants are valid YAML
        let configs = [
            configs::MINIMAL,
            configs::README_ONLY,
            configs::WITH_EXCLUDE,
            configs::WITH_REPO,
        ];

        for config in configs {
            serde_yaml::from_str::<serde_yaml::Value>(config).expect("Config should be valid YAML");
        }
    }

    #[test]
    fn test_invalid_yaml_is_actually_invalid() {
        let result = serde_yaml::from_str::<serde_yaml::Value>(configs::INVALID_YAML);
        assert!(result.is_err(), "INVALID_YAML should not parse");
    }
}
