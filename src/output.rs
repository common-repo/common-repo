//! # Output Configuration
//!
//! This module provides utilities for controlling CLI output appearance,
//! including color and emoji support based on terminal capabilities and
//! user preferences.
//!
//! ## Respecting User Preferences
//!
//! The module respects the following environment variables and flags:
//! - `--color=never|always|auto` - CLI flag for color control
//! - `NO_COLOR` - Disables colors when set (per https://no-color.org/)
//! - `CLICOLOR=0` - Disables colors
//! - `CLICOLOR_FORCE=1` - Forces colors even in non-TTY
//! - `TERM=dumb` - Disables colors for dumb terminals
//!
//! ## Usage
//!
//! ```rust,ignore
//! use common_repo::output::{OutputConfig, emoji};
//!
//! let config = OutputConfig::from_env_and_flag("auto");
//!
//! // Use emoji helper that respects config
//! println!("{} Processing...", emoji(&config, "🔍", "[SCAN]"));
//! ```

use std::env;

/// Output configuration for controlling colors and emojis.
#[derive(Debug, Clone)]
pub struct OutputConfig {
    /// Whether colors and emojis should be used in output.
    pub use_color: bool,
}

impl OutputConfig {
    /// Create an output configuration from environment and CLI flag.
    ///
    /// # Arguments
    /// * `color_flag` - The value of the --color CLI flag: "always", "never", or "auto"
    ///
    /// # Behavior
    /// - `--color=always`: Force colors on (overrides NO_COLOR)
    /// - `--color=never`: Force colors off
    /// - `--color=auto`: Detect based on environment
    ///
    /// In auto mode, colors are disabled if:
    /// - `NO_COLOR` environment variable is set (any value, including empty)
    /// - `CLICOLOR=0` is set
    /// - `TERM=dumb` is set
    /// - stdout is not a TTY (unless `CLICOLOR_FORCE=1`)
    pub fn from_env_and_flag(color_flag: &str) -> Self {
        let use_color = match color_flag.to_lowercase().as_str() {
            "always" => true,
            "never" => false,
            _ => Self::detect_color_support(),
        };

        Self { use_color }
    }

    /// Detect whether color output is supported based on environment.
    fn detect_color_support() -> bool {
        // Check NO_COLOR first (https://no-color.org/)
        // The presence of the variable (even if empty) disables colors
        if env::var_os("NO_COLOR").is_some() {
            return false;
        }

        // Check CLICOLOR=0 disables colors
        if env::var("CLICOLOR").is_ok_and(|v| v == "0") {
            return false;
        }

        // Check CLICOLOR_FORCE=1 forces colors
        if env::var("CLICOLOR_FORCE").is_ok_and(|v| v != "0" && !v.is_empty()) {
            return true;
        }

        // Check TERM=dumb
        if env::var("TERM").is_ok_and(|v| v == "dumb") {
            return false;
        }

        // Use console crate's detection for TTY and color support
        console::Term::stdout().features().colors_supported()
    }

    /// Create a configuration with colors always enabled.
    #[cfg(test)]
    pub fn with_color() -> Self {
        Self { use_color: true }
    }

    /// Create a configuration with colors always disabled.
    #[cfg(test)]
    pub fn without_color() -> Self {
        Self { use_color: false }
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self::from_env_and_flag("auto")
    }
}

/// Returns the appropriate string based on color configuration.
///
/// When colors are enabled, returns the emoji. When disabled, returns
/// the plain text alternative.
///
/// # Arguments
/// * `config` - The output configuration
/// * `emoji` - The emoji to use when colors are enabled
/// * `plain` - The plain text to use when colors are disabled
///
/// # Example
/// ```rust,ignore
/// let config = OutputConfig::from_env_and_flag("auto");
/// println!("{} Validating...", emoji(&config, "🔍", "[SCAN]"));
/// ```
pub fn emoji<'a>(config: &OutputConfig, emoji_str: &'a str, plain: &'a str) -> &'a str {
    if config.use_color {
        emoji_str
    } else {
        plain
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_always() {
        let config = OutputConfig::from_env_and_flag("always");
        assert!(config.use_color);
    }

    #[test]
    fn test_color_never() {
        let config = OutputConfig::from_env_and_flag("never");
        assert!(!config.use_color);
    }

    #[test]
    fn test_emoji_helper_with_color() {
        let config = OutputConfig::with_color();
        assert_eq!(emoji(&config, "🔍", "[SCAN]"), "🔍");
    }

    #[test]
    fn test_emoji_helper_without_color() {
        let config = OutputConfig::without_color();
        assert_eq!(emoji(&config, "🔍", "[SCAN]"), "[SCAN]");
    }
}
