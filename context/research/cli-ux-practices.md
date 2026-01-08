# CLI/UX Research

Research compiled from OSS recommendations (2024-2025) and analysis of exemplar Rust CLI projects.

## Reference Projects Analyzed

1. **uv** (astral-sh/uv) - Python package manager with progress bars and spinners
2. **ruff** (astral-sh/ruff) - Python linter with clear error output
3. **ripgrep** (BurntSushi/ripgrep) - Search tool with excellent error messages
4. **gh** (cli/cli) - GitHub CLI with accessibility focus
5. **starship** (starship/starship) - Cross-shell prompt with color customization

## Core CLI Design Philosophy

### Human-First Design

From [Command Line Interface Guidelines](https://clig.dev/), modern CLIs should prioritize:

1. **Humans over machines**: Design for interactive users first, maintain composability second
2. **Simple composability**: Use stdin/stdout/stderr, exit codes, and plain text
3. **Consistency**: Follow established terminal conventions users expect
4. **Discoverability**: Make commands learnable, not memorization-dependent
5. **Conversational interaction**: Design for iterative command refinement
6. **Empathy**: Make users feel supported at every interaction point

### Robustness Principles

Software must feel solid through:
- **Responsiveness**: Output within 100ms, even if just a status message
- **Validation**: Check input early, before state changes
- **Progress feedback**: Show activity for any operation over ~100ms
- **Recovery**: Enable idempotent operations for easy retry
- **Graceful interruption**: Ctrl-C must exit quickly

## Help Text Guidelines

### Structure and Format

```
Usage: myapp <command> [options]

A brief description of what the tool does.

Commands:
  build    Build the project
  test     Run tests
  deploy   Deploy to production

Options:
  -h, --help     Show this help message
  -v, --version  Show version
  -q, --quiet    Suppress output

Examples:
  myapp build --release
  myapp test --filter unit

Run 'myapp <command> --help' for more information on a specific command.
```

### Recommendations

1. **Accept multiple help forms**: `-h`, `--help`, and `help` subcommand
2. **Lead with examples**: Users prefer working samples over abstract explanations
3. **Keep initial output brief**: Description, 1-2 examples, essential flags
4. **Show common commands first**: Don't overwhelm with exhaustive lists
5. **Suggest corrections**: "Did you mean 'upgrade'?" for typos
6. **Include support paths**: Website or GitHub links in top-level help
7. **Format for scannability**: Use sections and consistent spacing

### Clap-Specific Patterns

Using the [clap](https://github.com/clap-rs/clap) derive macro:

```rust
use clap::{Parser, Subcommand};

/// A fast package manager for Python
#[derive(Parser)]
#[command(name = "myapp")]
#[command(version, about, long_about = None)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Configuration file path
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build the project
    Build {
        /// Build in release mode
        #[arg(short, long)]
        release: bool,
    },
}
```

**Naming conventions** (Unix/GNU):
- Commands and arguments in lowercase kebab-case: `--example-opt`
- Short flags for frequently-used options only: `-v`, `-h`

## Error Message Guidelines

### User-Friendly Error Design

From research on Rust CLI error handling:

1. **Rewrite errors for humans**: Catch expected errors and explain conversationally
2. **Add context**: Use `anyhow::Context` or similar to add situational information
3. **Suggest fixes**: Include practical remediation when possible
4. **Keep messages lowercase**: Without trailing punctuation
5. **Separate from cause**: Error messages describe the current error only

### Error Message Format

```
error: can't write to file.txt

cause: permission denied (os error 13)

hint: Try running 'chmod +w file.txt' or run with sudo
```

### Library Choice for Rust CLIs

**Decision tree**:
```
Should caller handle error variants differently?
├── Yes → Use thiserror (enum with variants)
└── No → Is this library or application code?
    ├── Library → Use thiserror (opaque or simple)
    └── Application → Use anyhow
        └── Want pretty errors? → Use color-eyre
```

**thiserror** - For libraries with structured errors:
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("configuration file not found: {path}")]
    NotFound { path: PathBuf },

    #[error("invalid configuration format")]
    InvalidFormat(#[from] toml::de::Error),
}
```

**anyhow** - For applications focused on error reporting:
```rust
use anyhow::{Context, Result};

fn load_config(path: &Path) -> Result<Config> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read config from {}", path.display()))?;

    toml::from_str(&content)
        .context("failed to parse config")
}
```

**color-eyre** - For pretty error reports with backtraces:
```rust
use color_eyre::eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;
    // ... application code
}
```

### Signal-to-Noise Ratio

- **Group similar errors**: Use headers rather than repeating identical messages
- **Place critical info at end**: Eyes naturally focus on the last output
- **Use color sparingly**: Red for genuine errors only
- **Debug info for unexpected errors only**: Include bug reporting instructions

### ripgrep Error Patterns

From [ripgrep](https://github.com/BurntSushi/ripgrep):
- Use `--debug` flag to provide detailed troubleshooting info
- Suggest flags when patterns indicate user intent (e.g., suggest `-U/--multiline` for patterns with `\n`)
- Give descriptive messages that include the problematic file path

## Progress Indicators

### Pattern Selection

From [Evil Martians CLI UX research](https://evilmartians.com/chronicles/cli-ux-best-practices-3-patterns-for-improving-progress-displays):

| Pattern | When to Use | Example |
|---------|-------------|---------|
| **Spinner** | Unknown duration, single task | `⣾ Installing...` |
| **X of Y** | Known count, discrete items | `[3/10] Installing package` |
| **Progress Bar** | Known size/percentage | `[████████░░░░] 67%` |

### indicatif Library

The [indicatif](https://github.com/console-rs/indicatif) crate is the standard for Rust progress bars:

```rust
use indicatif::{ProgressBar, ProgressStyle};

// Simple spinner
let spinner = ProgressBar::new_spinner();
spinner.enable_steady_tick(Duration::from_millis(100));
spinner.set_message("Loading...");

// Progress bar with count
let pb = ProgressBar::new(total);
pb.set_style(ProgressStyle::default_bar()
    .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
    .unwrap()
    .progress_chars("#>-"));

for item in items {
    // process item
    pb.inc(1);
}
pb.finish_with_message("Done");
```

### Recommendations

1. **Auto-hide for non-TTY**: indicatif hides progress when output is piped
2. **Steady tick for slow operations**: Background thread keeps spinner moving
3. **Multi-progress for parallel tasks**: Use `MultiProgress` for concurrent operations
4. **Finish explicitly**: Call `finish_with_message` to clean up display
5. **Performance**: Default 20fps is usually sufficient; don't update too frequently
6. **Integration**: Use `indicatif-log-bridge` to coordinate with logging

### uv/ruff Progress Patterns

From [astral-sh/uv](https://github.com/astral-sh/uv):
- Support `UV_NO_PROGRESS` environment variable to disable spinners/bars
- Show progress for downloads of large packages
- Keep progress minimal but informative

## Color Usage Guidelines

### When to Use Color

1. **Highlight important information**: Errors, warnings, success states
2. **Distinguish categories**: Different types of output
3. **Guide attention**: Not decorative, but functional

### When to Disable Color

Automatically disable color when:
- **stdout/stderr is not a TTY** (piping to file/other programs)
- **`NO_COLOR` environment variable** is set (any value)
- **`TERM=dumb`** is set
- **User passes `--no-color`** flag
- Consider app-specific variable: `MYAPP_NO_COLOR`

### Implementation

Using [termcolor](https://github.com/BurntSushi/termcolor) (cross-platform):

```rust
use std::io::{IsTerminal, Write};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

fn main() {
    let choice = if std::io::stdout().is_terminal() {
        ColorChoice::Auto
    } else {
        ColorChoice::Never
    };

    let mut stdout = StandardStream::stdout(choice);
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
    writeln!(&mut stdout, "Error: something went wrong")?;
    stdout.reset()?;
}
```

Using [colored](https://github.com/colored-rs/colored) (simpler API):

```rust
use colored::*;

println!("{}", "Error:".red().bold());
println!("{}", "Success!".green());

// Respects NO_COLOR automatically
// Can override with colored::control::set_override(false)
```

### Color Palette Recommendations

- **Red**: Errors, failures
- **Yellow/Orange**: Warnings, deprecations
- **Green**: Success, completion
- **Cyan/Blue**: Info, progress, paths
- **Bold**: Emphasis, commands to run
- **Dim/Gray**: Secondary information, hints

### Accessibility Considerations

From [GitHub CLI accessibility work](https://github.blog/engineering/user-experience/building-a-more-accessible-github-cli/):
- Color alone should not convey information (use text labels too)
- Ensure sufficient contrast for readability
- Replace animated spinners with static text for screen readers
- Provide contextual messages describing actions

## Output Formatting

### Human vs Machine Output

1. **Check TTY for formatting decisions**: Use `std::io::IsTerminal`
2. **Primary output to stdout**: Include machine-readable data
3. **Logs and errors to stderr**: Keeps stdout clean for piping
4. **Support `--json` flag**: Structured output for automation
5. **Support `--plain` flag**: Disable formatting that breaks line parsing

### Display Guidelines

1. **Show output on success**: Avoid UNIX silence convention for interactive use
2. **Explain state changes**: Describe what modifications occurred
3. **Show current state easily**: Commands like `git status` reveal conditions
4. **Suggest next commands**: Guide workflows within output

### JSON Output Pattern

```rust
use serde::Serialize;

#[derive(Serialize)]
struct Output {
    status: String,
    files_processed: usize,
    errors: Vec<String>,
}

fn output_result(result: &Output, json: bool) {
    if json {
        println!("{}", serde_json::to_string_pretty(result).unwrap());
    } else {
        println!("Status: {}", result.status);
        println!("Files: {}", result.files_processed);
        for err in &result.errors {
            eprintln!("Error: {}", err);
        }
    }
}
```

## Interactivity Guidelines

### Prompt Guidelines

1. **Only prompt when stdin is TTY**: Fail clearly in scripts
2. **Never require prompts**: Always offer flag/argument equivalents
3. **Respect `--no-input`/`--yes`**: Disable interactive elements
4. **Don't echo passwords**: Disable terminal echo during input
5. **Make exit clear**: Ensure Ctrl-C always works

### Confirmation Patterns

| Risk Level | Behavior |
|------------|----------|
| Low (delete temp file) | No confirmation needed |
| Medium (delete user file) | Prompt with y/n, or `--force` flag |
| High (delete production DB) | Require typing confirmation text |

Example implementation:
```rust
use dialoguer::Confirm;

if force || Confirm::new()
    .with_prompt("Delete all files?")
    .default(false)
    .interact()?
{
    delete_files()?;
}
```

## Configuration Patterns

### Configuration File Design

From ripgrep and starship examples:

1. **Environment variable for config path**: `MYAPP_CONFIG_PATH`
2. **Simple format**: One argument per line, `#` for comments
3. **TOML for structured config**: Easy to read and write
4. **XDG compliance**: `~/.config/myapp/config.toml` on Linux

### Example: Starship Configuration

```toml
# ~/.config/starship.toml

[directory]
truncation_length = 3
format = "[$path]($style) "

[git_branch]
symbol = " "
format = "[$symbol$branch]($style) "
```

## Shell Completions

### Generation Patterns

Using clap's `clap_complete`:

```rust
use clap::CommandFactory;
use clap_complete::{generate, Shell};

fn generate_completions(shell: Shell) {
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "myapp", &mut std::io::stdout());
}
```

### Recommendations

1. **Support major shells**: Bash, Zsh, Fish, PowerShell
2. **Include in installation**: Provide setup instructions
3. **Consider config file**: Like ripgrep, include config values in completions
4. **Runtime completions**: For values like branch names, file paths

## Exemplar Project Patterns

### ripgrep

- **Excellent error messages**: Specific, practical, includes file paths
- **Debug mode**: `--debug` for troubleshooting
- **Smart suggestions**: Recommends flags based on pattern analysis
- **Binary file handling**: Clear UX for searching binary files

### gh (GitHub CLI)

- **Accessibility focus**: Static progress indicators for screen readers
- **Helpful error messages**: Context-aware suggestions
- **Interactive prompts**: Only when TTY, with flag alternatives
- **Subcommand structure**: Organized by resource type

### starship

- **Extensive customization**: TOML configuration
- **Color theming**: Standard terminal colors with hex support
- **Cross-platform**: Works across shells
- **Minimal defaults**: Information appears only when relevant

### uv/ruff

- **Speed focus**: Performance messaging reinforces brand
- **Progress control**: `UV_NO_PROGRESS` environment variable
- **Clear output**: Minimal but informative status updates

## Summary

### Help Text
1. Lead with examples, not abstract descriptions
2. Accept `-h`, `--help`, and `help` subcommand
3. Show common commands first, link to detailed docs

### Error Messages
1. Rewrite technical errors for humans
2. Include context and suggested fixes
3. Use color-eyre or anyhow for pretty backtraces

### Progress Indicators
1. Use indicatif for progress bars/spinners
2. Auto-hide when not TTY
3. Support environment variable to disable

### Colors
1. Disable when not TTY, when NO_COLOR set, when TERM=dumb
2. Use color for function, not decoration
3. Don't rely on color alone for information

### Output
1. stdout for data, stderr for messages
2. Support --json for automation
3. Show success messages, suggest next steps

## Sources

### CLI Design Guidelines
- [Command Line Interface Guidelines](https://clig.dev/)
- [Thoughtworks - CLI Design Guidelines](https://www.thoughtworks.com/insights/blog/engineering-effectiveness/improve-developer-experiences-cli-design-guidelines)
- [Evil Martians - CLI UX Patterns](https://evilmartians.com/chronicles/cli-ux-best-practices-3-patterns-for-improving-progress-displays)
- [Ubuntu - Command-line Usability](https://ubuntu.com/blog/command-line-usability-a-terminal-users-thought-process)

### Rust CLI Libraries
- [clap GitHub](https://github.com/clap-rs/clap)
- [indicatif GitHub](https://github.com/console-rs/indicatif)
- [termcolor GitHub](https://github.com/BurntSushi/termcolor)
- [colored GitHub](https://github.com/colored-rs/colored)
- [dialoguer docs](https://docs.rs/dialoguer/latest/dialoguer/)

### Error Handling
- [thiserror, anyhow, or How I Handle Errors](https://www.shakacode.com/blog/thiserror-anyhow-or-how-i-handle-errors-in-rust-apps/)
- [Error Handling for Large Rust Projects](https://greptime.com/blogs/2024-05-07-error-rust)
- [Error Handling In Rust](https://lpalmieri.com/posts/error-handling-rust/)
- [Effective Error Handling in Rust CLI Apps](https://technorely.com/insights/effective-error-handling-in-rust-cli-apps-best-practices-examples-and-advanced-techniques)

### Reference Projects
- [ripgrep GitHub](https://github.com/BurntSushi/ripgrep)
- [uv GitHub](https://github.com/astral-sh/uv)
- [ruff GitHub](https://github.com/astral-sh/ruff)
- [gh GitHub](https://github.com/cli/cli)
- [starship GitHub](https://github.com/starship/starship)

### Accessibility
- [Building a more accessible GitHub CLI](https://github.blog/engineering/user-experience/building-a-more-accessible-github-cli/)
