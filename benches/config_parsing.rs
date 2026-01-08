//! Benchmarks for configuration parsing operations.
//!
//! These benchmarks measure the performance of parsing `.common-repo.yaml`
//! configuration files of various sizes and complexity levels.

use common_repo::config;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

/// Minimal configuration with a single include operation.
const MINIMAL_CONFIG: &str = r#"
- include:
    - "*.rs"
"#;

/// Small configuration with basic operations.
const SMALL_CONFIG: &str = r#"
- repo:
    url: https://github.com/example/repo.git
    ref: main
- include:
    - "src/**"
    - "*.rs"
- exclude:
    - "target/**"
    - ".git/**"
"#;

/// Medium configuration with multiple operations and nesting.
const MEDIUM_CONFIG: &str = r#"
- repo:
    url: https://github.com/example/repo.git
    ref: main
    with:
      - include:
          - "src/**"
          - "*.rs"
      - exclude:
          - "target/**"
- repo:
    url: https://github.com/example/other.git
    ref: v1.0.0
    with:
      - include:
          - "lib/**"
      - rename:
          - "old/(.*)": "new/%[1]s"
- include:
    - "docs/**"
    - "README.md"
- exclude:
    - "**/*.tmp"
    - "**/*.log"
- template:
    - "templates/**"
- template-vars:
    project: myproject
    version: 1.0.0
"#;

/// Complex configuration with deep nesting and many operations.
const COMPLEX_CONFIG: &str = r#"
- repo:
    url: https://github.com/example/base.git
    ref: main
    with:
      - repo:
          url: https://github.com/example/nested.git
          ref: main
          with:
            - include:
                - "src/**"
            - exclude:
                - "target/**"
      - include:
          - "docs/**"
      - exclude:
          - "*.tmp"
- repo:
    url: https://github.com/example/other.git
    ref: v2.0.0
    path: src
    with:
      - include:
          - "**/*"
      - exclude:
          - ".git/**"
      - rename:
          - "src/([^/]+)/([^/]+)/(.*)": "%[2]s/%[1]s/%[3]s"
          - "old_(.*)": "new_%[1]s"
- include:
    - "src/**"
    - "tests/**"
    - "*.rs"
    - "Cargo.toml"
- exclude:
    - "**/*.swp"
    - "**/*.tmp"
    - "**/*.log"
    - ".git/**"
    - "target/**"
- template:
    - "templates/**"
    - "*.template"
    - "config/*.tmpl"
- rename:
    - "src/([^/]+)/([^/]+)/(.*)": "%[2]s/%[1]s/%[3]s"
    - "^old/(.*)": "new/%[1]s"
    - "(.+)\\.bak$": "%[1]s"
- tools:
    - rustc: ">=1.70"
    - cargo: "*"
    - pre-commit: "*"
- template-vars:
    project_name: benchmark-project
    version: 1.0.0
    author: test-author
    description: A test project for benchmarking
"#;

fn generate_large_config(num_repos: usize, patterns_per_op: usize) -> String {
    let mut config = String::new();

    for i in 0..num_repos {
        config.push_str(&format!(
            r#"- repo:
    url: https://github.com/example/repo-{}.git
    ref: main
    with:
"#,
            i
        ));

        config.push_str("      - include:\n");
        for j in 0..patterns_per_op {
            config.push_str(&format!("          - \"src{}/module{}/**\"\n", i, j));
        }

        config.push_str("      - exclude:\n");
        for j in 0..patterns_per_op {
            config.push_str(&format!("          - \"**/*.tmp{}\"\n", j));
        }
    }

    // Add global operations
    config.push_str("- include:\n");
    for i in 0..patterns_per_op {
        config.push_str(&format!("    - \"global/pattern{}/**\"\n", i));
    }

    config.push_str("- exclude:\n");
    for i in 0..patterns_per_op {
        config.push_str(&format!("    - \"**/*.exclude{}\"\n", i));
    }

    config
}

fn bench_config_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_parsing");

    // Benchmark different config sizes
    group.bench_function("minimal", |b| {
        b.iter(|| config::parse(black_box(MINIMAL_CONFIG)))
    });

    group.bench_function("small", |b| {
        b.iter(|| config::parse(black_box(SMALL_CONFIG)))
    });

    group.bench_function("medium", |b| {
        b.iter(|| config::parse(black_box(MEDIUM_CONFIG)))
    });

    group.bench_function("complex", |b| {
        b.iter(|| config::parse(black_box(COMPLEX_CONFIG)))
    });

    group.finish();
}

fn bench_config_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_scaling");

    // Test scaling with number of repos
    for num_repos in [5, 10, 20, 50] {
        let config = generate_large_config(num_repos, 5);
        group.bench_with_input(
            BenchmarkId::new("repos", num_repos),
            &config,
            |b, config| b.iter(|| config::parse(black_box(config))),
        );
    }

    // Test scaling with patterns per operation
    for patterns in [5, 10, 20, 50] {
        let config = generate_large_config(5, patterns);
        group.bench_with_input(
            BenchmarkId::new("patterns", patterns),
            &config,
            |b, config| b.iter(|| config::parse(black_box(config))),
        );
    }

    group.finish();
}

criterion_group!(benches, bench_config_parsing, bench_config_scaling);
criterion_main!(benches);
