# common-repo: Purpose and Goals

## The Problem

Modern software repositories require extensive configuration and tooling infrastructure that isn't actually source code: CI/CD pipelines, pre-commit hooks, linters, formatters, dependency management configs, and countless other dotfiles. Currently, developers face several challenges:

1. **Manual copypasta**: Developers copy configuration files between projects, leading to inconsistency and drift
2. **Lack of versioning**: Unlike source dependencies, these configuration files aren't semantically versioned or tracked as dependencies
3. **Difficult updates**: When best practices evolve or security patches are needed, there's no automated way to propagate changes across repositories
4. **No inheritance model**: Repositories can't easily build upon or extend standard configurations - it's all-or-nothing copying

Existing tools like cookiecutter, copier, and repository templates have drawbacks and lack the features needed for a complete solution.

## The Solution

**common-repo is a tool that treats repository configuration files as software dependencies.**

With common-repo, configuration files, CI/CD definitions, supporting scripts, and other non-source-code infrastructure become:
- **Semantically versioned** - Track exactly which version of configurations you're using
- **Automatically updateable** - Detect when inherited configs are out of date and upgrade deterministically
- **Composable** - Pull in multiple configuration sources and merge them intelligently
- **Inheritable** - Build upon standard configurations, which themselves can build upon other standards

## How It Works

Repositories define their configuration dependencies in a `.common-repo.yaml` file. This single file:
1. Declares which remote repositories to pull configuration from
2. Pins specific versions using git refs (tags, branches, commits)
3. Defines operations to composite, merge, and customize the resulting files

The tool operates on an in-memory filesystem, applying operations in order:
- **repo**: Pull files from remote repositories (which can themselves reference other repos, creating inheritance chains)
- **include/exclude**: Add or remove files based on patterns
- **rename**: Transform file paths deterministically
- **template**: Process files with template variables
- **yaml/json/toml/ini/markdown**: Intelligently merge configuration fragments into structured files

Because operations are ordered and deterministic, conflicts are resolved predictably. The same `.common-repo.yaml` always produces the same output.

## Design Philosophy

### Language and Platform Agnostic
The core tool handles filesystem composition generically - it doesn't care what programming language or platform you're using. However, smart merging and templating focuses on common configuration file formats (YAML, JSON, TOML, INI, Markdown) rather than source code files.

### Semantic Versioning of Refs
Pinned repository refs can be checked against published tags and releases. The tool can detect when newer versions are available (including breaking changes) and suggest or automate version bumps.

### Composability Over Monoliths
Rather than creating massive, all-encompassing templates, common-repo encourages small, focused configuration repositories that can be composed together. Want a Rust CLI with semantic versioning and Python tooling? Pull in `common-repo/rust-cli`, `common-repo/semantic-versioning`, and `common-repo/python-uv` - they'll merge cleanly.

## Goals

### Primary Goals

1. **Eliminate configuration copypasta** - No more copying .pre-commit-config.yaml between projects
2. **Enable automatic updates** - Security patches and improvements propagate automatically
3. **Standardize best practices** - The common-repo organization publishes opinionated, best-practice configurations for common platforms
4. **Support customization** - Organizations and individuals can derive or create their own standard repositories

### Target Audiences

**Initially**: Organizations wanting to standardize repositories across teams

**Long-term**: The entire developer community - making it dead simple to bootstrap a repository with best practices and maintain it over time

### Success Metrics

A successful common-repo deployment means:
- Developers spend seconds, not hours, setting up new repositories
- Configuration updates propagate across all repositories automatically
- Every repository in an organization follows current best practices
- The community builds and shares reusable configuration standards

## Non-Goals

- Managing source code files (though the tool doesn't prevent this, it's not the focus)
- Smart merging of language-specific source files (.py, .rs, .go, etc.)
- Replacing package managers for source dependencies
- Providing IDE integrations or editor tooling

## Vision

We envision a future where repository infrastructure is as well-managed as source code dependencies. Just as developers don't manually copy libraries into their projects, they shouldn't manually copy CI/CD configs. Just as security updates can be applied to dependencies automatically, they should apply to repository configurations too.

The common-repo ecosystem aims to standardize software development practices, making quality tooling and best practices accessible to everyone - from individual hobbyists to large organizations.
