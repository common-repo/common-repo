# Distribution Guidelines for Rust CLI Tools

This document summarizes distribution strategies and recommendations for Rust CLI tools, based on research of popular projects and community guidance.

## Exemplar Projects Analyzed

- **uv** (astral-sh) - Python package manager
- **ruff** (astral-sh) - Python linter
- **ripgrep** (BurntSushi) - Fast grep alternative
- **starship** - Cross-shell prompt

## Distribution Philosophy

The consensus across successful Rust CLI projects is a **multi-channel distribution approach** that meets users where they are. Rather than choosing one method, mature projects offer multiple installation paths to reduce adoption barriers across different user profiles and technical preferences.

### Recommended Progression

The [Command Line Applications in Rust book](https://rust-cli.github.io/book/tutorial/packaging.html) recommends a layered approach:

1. **Start with `cargo install`** - Simplest for Rust developers
2. **Add binary releases** - Pre-compiled binaries via GitHub Releases
3. **Distribute via package managers** - Homebrew, apt, AUR, etc.

This progression matches project maturity: early projects can start simple, adding channels as the user base grows.

## Distribution Channels

### 1. Cargo Install / crates.io

**Pros:**
- Zero setup for publication
- Built-in to Rust toolchain
- Version management via Cargo.toml

**Cons:**
- Requires Rust toolchain installed
- Compiles from source (slow)
- System dependencies may be required

**Best for:** Tools targeted at Rust developers, cargo subcommands.

### 2. Pre-compiled Binaries (GitHub Releases)

**Pros:**
- No compilation required
- Instant installation
- No Rust dependency

**Cons:**
- Requires building for each platform
- More CI/CD complexity
- Users must manage updates

**Implementation approaches:**
- Manual GitHub Releases
- CI/CD with matrix builds
- [cargo-dist](https://opensource.axo.dev/cargo-dist/) (recommended)

### 3. Shell/PowerShell Installers

Common pattern used by uv, ruff, starship:

```bash
# macOS/Linux
curl -LsSf https://example.com/install.sh | sh

# Windows
powershell -ExecutionPolicy ByPass -c "irm https://example.com/install.ps1 | iex"
```

**Advantages:**
- Works without Rust or package managers
- Single-command installation
- Can include self-update capability

**cargo-dist** generates these installers automatically.

### 4. cargo-binstall Support

[cargo-binstall](https://github.com/cargo-bins/cargo-binstall) enables faster installation by downloading pre-built binaries instead of compiling:

```bash
cargo binstall my-tool  # Downloads binary instead of compiling
```

**Benefits:**
- ~10x faster than `cargo install`
- Works with existing crates.io metadata
- Automatic GitHub Release detection

To support cargo-binstall, simply publish pre-built binaries to GitHub Releases with standard naming conventions.

### 5. System Package Managers

| Platform | Package Manager | Tools |
|----------|----------------|-------|
| macOS | Homebrew | `brew install` |
| macOS | MacPorts | `port install` |
| Linux (Debian/Ubuntu) | APT | `apt install` |
| Linux (Arch) | pacman/AUR | `pacman -S` / `yay` |
| Linux (Fedora) | DNF | `dnf install` |
| Windows | Chocolatey | `choco install` |
| Windows | Scoop | `scoop install` |
| Windows | winget | `winget install` |
| Cross-platform | Conda | `conda install` |
| Cross-platform | Nix | `nix-env -i` |

**Trade-offs:**
- Maximum accessibility
- Automatic updates
- Each package manager requires maintenance
- Version lag as packages are reviewed

### 6. Language-Specific Registries

For tools targeting specific communities:

- **PyPI** (pip/pipx) - Python tools like ruff, uv
- **npm** - JavaScript/Node tools
- **Homebrew taps** - Custom Homebrew repositories

## Cross-Compilation Strategies

### Target Platforms

Standard target matrix for broad coverage:

| OS | Architecture | Target Triple |
|----|--------------|---------------|
| Linux | x86_64 | `x86_64-unknown-linux-gnu` |
| Linux | x86_64 (static) | `x86_64-unknown-linux-musl` |
| Linux | ARM64 | `aarch64-unknown-linux-gnu` |
| Linux | ARM64 (static) | `aarch64-unknown-linux-musl` |
| macOS | x86_64 | `x86_64-apple-darwin` |
| macOS | ARM64 | `aarch64-apple-darwin` |
| Windows | x86_64 | `x86_64-pc-windows-msvc` |

### Cross-Compilation Tools

1. **Native runners** (GitHub Actions)
   - Use platform-specific runners for each target
   - Avoids cross-compilation complexity
   - Recommended for most projects

2. **[cross](https://github.com/cross-rs/cross)**
   - Docker-based cross-compilation
   - Same CLI as cargo
   - Supports many Linux targets

3. **[cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild)**
   - Uses Zig as a linker
   - Linux and macOS cross-compilation
   - Supported by cargo-dist

4. **[cargo-xwin](https://github.com/rust-cross/cargo-xwin)**
   - Cross-compile to Windows from Linux
   - Uses MSVC CRT

### Static Linking with MUSL

For maximum Linux compatibility:

```bash
cargo build --target x86_64-unknown-linux-musl
```

**Benefits:**
- No libc dependencies
- Works on any Linux distribution
- Smaller container images (scratch/distroless)

**Docker builders:**
- [clux/muslrust](https://github.com/clux/muslrust)
- [emk/rust-musl-builder](https://github.com/emk/rust-musl-builder)

## Container Distribution

### Base Image Recommendations

| Image | Size | Scenario |
|-------|------|----------|
| `scratch` | 0 MB | Fully static binaries |
| `gcr.io/distroless/static` | ~2 MB | Static + CA certs + non-root user |
| `gcr.io/distroless/cc` | ~20 MB | Dynamically linked binaries |
| `alpine` | ~5 MB | When shell access needed |
| `cgr.dev/chainguard/static` | ~2 MB | Distroless alternative |

**Best practice:** Use `distroless/static` or `chainguard/static` for production - they include SSL certs and non-root users unlike scratch.

### Multi-Stage Build Pattern

```dockerfile
FROM rust:1.XX as builder
WORKDIR /app
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM gcr.io/distroless/static
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/myapp /
ENTRYPOINT ["/myapp"]
```

## Release Automation with cargo-dist

[cargo-dist](https://github.com/axodotdev/cargo-dist) is the recommended tool for automating Rust binary releases:

**Features:**
- Generates GitHub Actions workflows
- Builds binaries for multiple platforms
- Creates shell/PowerShell installers
- Supports cargo-binstall
- Generates checksums and SBOMs
- Handles GitHub Release creation

**Recent improvements (2024):**
- Extended cross-compilation to Linux (cargo-zigbuild) and Windows (cargo-xwin)
- Shell installer now validates checksums
- CycloneDX SBOM support
- New config format (`dist-workspace.toml`)

**Quick start:**
```bash
cargo install cargo-dist
cargo dist init
```

This generates `.github/workflows/release.yml` that handles the full release pipeline.

## Distribution Patterns from Exemplar Projects

### uv (Astral)

- Shell/PowerShell installers (primary)
- PyPI (pip/pipx)
- Self-update capability (`uv self update`)
- No prerequisites (works without Rust or Python)

### ruff (Astral)

- Shell/PowerShell installers
- PyPI (pip/pipx)
- Homebrew
- Conda
- Version-specific installer URLs

### ripgrep

- Pre-compiled binaries (GitHub Releases)
- cargo-binstall support
- Extensive package manager coverage (Homebrew, apt, pacman, etc.)
- Static Linux binaries

### starship

- Shell installer script
- cargo install
- Dozens of package managers
- MSI installers for Windows
- conda-forge

## Summary

1. **Multi-channel is essential** - Offer multiple installation methods to maximize accessibility

2. **Shell installers reduce friction** - Single-command installation works for any user

3. **cargo-dist simplifies releases** - Handles cross-platform builds and installer generation

4. **Static binaries improve compatibility** - MUSL builds work everywhere

5. **cargo-binstall is low-effort** - Just publish standard GitHub Releases

6. **Package managers follow naturally** - Add them as demand grows

7. **Self-update is valuable** - Let users update without package managers

## References

- [Command Line Applications in Rust - Packaging](https://rust-cli.github.io/book/tutorial/packaging.html)
- [cargo-dist Documentation](https://opensource.axo.dev/cargo-dist/)
- [cargo-binstall](https://github.com/cargo-bins/cargo-binstall)
- [cross - Cross compilation](https://github.com/cross-rs/cross)
- [Distroless Container Images](https://github.com/GoogleContainerTools/distroless)
