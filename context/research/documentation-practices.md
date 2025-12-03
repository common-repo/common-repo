# Documentation Best Practices Research

Research compiled from OSS best practices (2024-2025) and analysis of exemplar Rust projects.

## Reference Projects Analyzed

1. **uv** (astral-sh/uv) - Python package manager
2. **ruff** (astral-sh/ruff) - Python linter/formatter
3. **ripgrep** (BurntSushi/ripgrep) - Line-oriented search tool
4. **starship** (starship/starship) - Cross-shell prompt
5. **tokio** (tokio-rs/tokio) - Async runtime

## README Best Practices

### Structure Pattern

All analyzed projects follow a consistent hierarchy:

1. **Header**: Logo/title, badges, one-line tagline
2. **Value proposition**: 3-6 key features with visual emphasis
3. **Installation**: Multiple methods (platform-specific, package managers)
4. **Quick start**: Immediate usage examples
5. **Documentation links**: External docs, guides, API reference
6. **Contributing**: Links to contribution guides
7. **License**: Clear licensing information

### Badges (Status Indicators)

Common badges across projects:
- Build/CI status (GitHub Actions)
- Version (crates.io / PyPI)
- License type
- Community link (Discord)
- Packaging status (repology)

**Pattern**: 4-6 badges positioned at the top, conveying project maturity and active maintenance.

### One-Line Tagline Examples

- **uv**: "An extremely fast Python package and project manager, written in Rust."
- **ruff**: "An extremely fast Python linter and code formatter, written in Rust."
- **ripgrep**: "ripgrep recursively searches directories for a regex pattern while respecting your gitignore"
- **tokio**: "A runtime for writing reliable, asynchronous, and slim applications"

**Pattern**: State what it does + key differentiator (speed, reliability, universality).

### Installation Section

All projects offer multiple installation paths:

| Project | Methods Offered |
|---------|-----------------|
| uv | Standalone installer, PyPI, Homebrew, cargo |
| ruff | uvx, pip, pipx, Homebrew, Conda, standalone |
| ripgrep | 20+ package managers, cargo, binaries |
| starship | curl installer, Homebrew, Scoop, cargo, 13+ shell configs |
| tokio | cargo only (library) |

**Pattern**: CLI tools provide 5+ installation methods; libraries use cargo.

### Feature Showcase

- **uv/ruff**: Emoji-marked bullet points with specific claims ("10-100x faster")
- **ripgrep**: Benchmark tables with detailed performance comparisons
- **starship**: Icon-based feature list emphasizing universality
- **tokio**: Three core strengths (Performance, Safety, Scalability)

**Pattern**: Either bullets with emojis/icons OR data-driven evidence (benchmarks).

## Documentation Site Patterns

### External Documentation

| Project | Docs URL | Platform |
|---------|----------|----------|
| uv | docs.astral.sh/uv | mkdocs-material |
| ruff | docs.astral.sh/ruff | mkdocs-material |
| ripgrep | GUIDE.md, FAQ.md in repo | In-repo markdown |
| starship | starship.rs | Custom |
| tokio | tokio.rs, docs.rs | Custom + rustdoc |

### Documentation Types

1. **Getting Started / Tutorials** - Onboarding new users
2. **User Guide** - Comprehensive feature documentation
3. **API Reference** - Auto-generated from code (rustdoc)
4. **FAQ** - Common questions and troubleshooting
5. **Contributing Guide** - Development setup, PR process
6. **Changelog** - Release history

### In-Repo vs External Docs

- **In-repo** (ripgrep pattern): GUIDE.md, FAQ.md, CHANGELOG.md
  - Pros: Version-controlled with code, no hosting needed
  - Cons: Less discoverable, no search, limited formatting

- **External docs site** (uv/ruff pattern): docs.astral.sh
  - Pros: Better navigation, search, cross-references
  - Cons: Requires hosting, can drift from code

**Recommendation**: External docs for complex tools, in-repo for focused utilities.

## Rust-Specific Documentation

### Rustdoc Best Practices

From [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/documentation.html):

1. **Crate-level docs**: Use `//!` at top of lib.rs/main.rs
   - Describe purpose and ecosystem fit
   - Setup instructions and common patterns
   - Link to examples

2. **Module docs**: Explain module's responsibility and key types

3. **Function/Type docs**: Use `///` with structure:
   - One-line summary
   - Detailed explanation
   - Code example
   - Standard sections: `# Examples`, `# Errors`, `# Panics`, `# Safety`

4. **Examples**: Doctests ensure examples stay correct
   ```rust
   /// Returns the sum of two numbers.
   ///
   /// # Examples
   ///
   /// ```
   /// let result = my_crate::add(2, 3);
   /// assert_eq!(result, 5);
   /// ```
   pub fn add(a: i32, b: i32) -> i32 { a + b }
   ```

### README and lib.rs Sync

Use `#![doc = include_str!("../README.md")]` in lib.rs to:
- Keep README and crate docs in sync
- Test README code examples via `cargo test`
- Single source of truth for overview documentation

## Documentation Tools (2024-2025)

### Popular Choices

1. **mdBook** - Rust ecosystem standard, gitbook-like
2. **mkdocs-material** - Feature-rich, used by uv/ruff
3. **Docusaurus** - React-based, good for larger docs
4. **rustdoc** - Built-in Rust API documentation

### Key Features to Look For

- Markdown support with extensions
- Search functionality
- Version control integration
- Mobile responsiveness
- Dark mode support
- Code syntax highlighting

## Community Validation

### Testimonials Pattern (ruff)

Ruff's README includes quotes from respected developers:
> "Ruff is so fast that sometimes I add an intentional bug... just to check that it's actually running." - Charlie Marsh

**Purpose**: Establishes credibility through social proof.

### Transparency Pattern (ripgrep)

Ripgrep includes "Why shouldn't I use ripgrep?" section:
> "If you need a portable and ubiquitous tool. While ripgrep works on Windows, macOS and Linux, it is not ubiquitous..."

**Purpose**: Builds trust by acknowledging limitations.

## Internationalization

### starship Pattern

- Documentation in 12+ languages
- Flag indicators for language selection
- Crowdin for community translations

**Consideration**: Important for broad adoption; adds maintenance burden.

## Key Takeaways

### README Must-Haves

1. Clear one-line description
2. Installation instructions (multiple methods for CLIs)
3. Basic usage example
4. Link to full documentation
5. License information
6. CI/build status badge

### Documentation Site Must-Haves

1. Getting started / quick start
2. Searchable content
3. API reference (auto-generated for Rust)
4. Examples section
5. Contributing guide

### Rust-Specific Requirements

1. Comprehensive rustdoc comments
2. Doctests for examples
3. README/lib.rs sync mechanism
4. Standard sections (Errors, Panics, Safety)

## Sources

### General OSS Documentation
- [Google Season of Docs - Docs Advisor Guide](https://opensource.googleblog.com/2025/05/introducing-new-open-source-documentation-resources.html)
- [Hackmamba - Top Open-Source Documentation Tools 2024](https://hackmamba.io/blog/2024/02/top-5-open-source-documentation-development-platforms-of-2024/)

### Rust Documentation
- [Rust API Guidelines - Documentation](https://rust-lang.github.io/api-guidelines/documentation.html)
- [The rustdoc book - How to write documentation](https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html)
- [Keeping README code examples up-to-date](https://blog.guillaume-gomez.fr/articles/2019-04-13+Keeping+Rust+projects'+README.md+code+examples+up-to-date)

### Reference Projects
- [uv GitHub Repository](https://github.com/astral-sh/uv)
- [ruff GitHub Repository](https://github.com/astral-sh/ruff)
- [ripgrep GitHub Repository](https://github.com/BurntSushi/ripgrep)
- [starship GitHub Repository](https://github.com/starship/starship)
- [tokio GitHub Repository](https://github.com/tokio-rs/tokio)
