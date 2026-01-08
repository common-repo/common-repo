//! Benchmarks for operations commonly used by operators.
//!
//! Since the operator modules are crate-private, these benchmarks test the
//! underlying operations that operators perform: glob matching on filesystems
//! and regex-based path renaming via the path module.

use common_repo::filesystem::MemoryFS;
use common_repo::path::regex_rename;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

/// Creates a MemoryFS simulating a typical project structure.
fn create_project_fs() -> MemoryFS {
    let mut fs = MemoryFS::new();

    // Rust source files
    for i in 0..50 {
        fs.add_file_string(format!("src/module{}/mod.rs", i), "// module")
            .unwrap();
        fs.add_file_string(format!("src/module{}/lib.rs", i), "// lib")
            .unwrap();
        fs.add_file_string(format!("src/module{}/tests.rs", i), "// tests")
            .unwrap();
    }

    // Test files
    for i in 0..30 {
        fs.add_file_string(format!("tests/test_{}.rs", i), "// test")
            .unwrap();
    }

    // Documentation
    for i in 0..20 {
        fs.add_file_string(format!("docs/guide/chapter{}.md", i), "# Chapter")
            .unwrap();
    }

    // Config files
    fs.add_file_string("Cargo.toml", "[package]").unwrap();
    fs.add_file_string("README.md", "# Project").unwrap();
    fs.add_file_string(".gitignore", "target/").unwrap();

    // Template files
    for i in 0..10 {
        fs.add_file_string(format!("templates/config{}.template", i), "config=${VAR}")
            .unwrap();
    }

    // Temporary/excluded files
    for i in 0..20 {
        fs.add_file_string(format!("target/debug/build{}.o", i), "binary")
            .unwrap();
        fs.add_file_string(format!("src/module{}/temp.tmp", i % 10), "temp")
            .unwrap();
    }

    fs
}

/// Benchmarks simulating include operator behavior.
///
/// The include operator uses `list_files_glob` to find matching files,
/// then copies them to the target filesystem.
fn bench_include_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("op_include");
    let source = create_project_fs();

    // Single pattern include
    group.bench_function("single_pattern", |b| {
        b.iter(|| {
            let mut target = MemoryFS::new();
            let matches = source.list_files_glob(black_box("src/**/*.rs")).unwrap();
            for path in &matches {
                if let Some(file) = source.get_file(path) {
                    target.add_file(path, file.clone()).unwrap();
                }
            }
            target
        })
    });

    // Multiple patterns include
    group.bench_function("multiple_patterns", |b| {
        b.iter(|| {
            let mut target = MemoryFS::new();
            let patterns = ["src/**/*.rs", "tests/**/*.rs", "docs/**/*.md"];
            for pattern in patterns {
                let matches = source.list_files_glob(black_box(pattern)).unwrap();
                for path in &matches {
                    if let Some(file) = source.get_file(path) {
                        target.add_file(path, file.clone()).unwrap();
                    }
                }
            }
            target
        })
    });

    // Wildcard include (all files)
    group.bench_function("wildcard", |b| {
        b.iter(|| {
            let mut target = MemoryFS::new();
            let matches = source.list_files_glob(black_box("**/*")).unwrap();
            for path in &matches {
                if let Some(file) = source.get_file(path) {
                    target.add_file(path, file.clone()).unwrap();
                }
            }
            target
        })
    });

    group.finish();
}

/// Benchmarks simulating exclude operator behavior.
///
/// The exclude operator uses `list_files_glob` to find matching files,
/// then removes them from the filesystem.
fn bench_exclude_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("op_exclude");

    // Single pattern exclude
    group.bench_function("single_pattern", |b| {
        b.iter_batched(
            create_project_fs,
            |mut fs| {
                let matches = fs.list_files_glob(black_box("target/**/*")).unwrap();
                for path in &matches {
                    fs.remove_file(path).unwrap();
                }
                fs
            },
            criterion::BatchSize::SmallInput,
        )
    });

    // Multiple patterns exclude
    group.bench_function("multiple_patterns", |b| {
        b.iter_batched(
            create_project_fs,
            |mut fs| {
                let patterns = ["target/**/*", "**/*.tmp", "**/*.o"];
                for pattern in patterns {
                    let matches = fs.list_files_glob(black_box(pattern)).unwrap();
                    for path in &matches {
                        fs.remove_file(path).unwrap();
                    }
                }
                fs
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

/// Benchmarks for regex-based path renaming.
///
/// The rename operator uses `regex_rename` from the path module
/// to transform file paths using capture groups.
fn bench_rename_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("op_rename");

    // Simple rename pattern
    group.bench_function("simple", |b| {
        b.iter(|| {
            regex_rename(
                black_box("old_(.*)"),
                black_box("new_%[1]s"),
                black_box("old_file.rs"),
            )
        })
    });

    // Multiple capture groups
    group.bench_function("multiple_captures", |b| {
        b.iter(|| {
            regex_rename(
                black_box("src/([^/]+)/([^/]+)/(.*)"),
                black_box("%[2]s/%[1]s/%[3]s"),
                black_box("src/module1/subdir/file.rs"),
            )
        })
    });

    // No match case
    group.bench_function("no_match", |b| {
        b.iter(|| {
            regex_rename(
                black_box("nonexistent/(.*)"),
                black_box("new/%[1]s"),
                black_box("src/module/file.rs"),
            )
        })
    });

    // Complex regex pattern
    group.bench_function("complex_regex", |b| {
        b.iter(|| {
            regex_rename(
                black_box("([a-zA-Z0-9_-]+)/([0-9]+)/(.*)"),
                black_box("%[3]s/v%[2]s/%[1]s"),
                black_box("module-name/123/deep/path/file.rs"),
            )
        })
    });

    group.finish();
}

/// Benchmarks for simulated rename operations on filesystems.
fn bench_rename_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("op_rename_fs");

    // Rename matching subset of files
    group.bench_function("partial_rename", |b| {
        b.iter_batched(
            create_project_fs,
            |mut fs| {
                let files: Vec<_> = fs.list_files();
                let pattern = "src/module([0-9]+)/(.*)";
                let replacement = "lib/mod%[1]s/%[2]s";

                for path in &files {
                    let path_str = path.to_string_lossy();
                    if let Ok(Some(new_name)) =
                        regex_rename(black_box(pattern), black_box(replacement), &path_str)
                    {
                        if new_name != path_str {
                            let new_path = std::path::Path::new(&new_name).to_path_buf();
                            fs.rename_file(path, &new_path).unwrap();
                        }
                    }
                }
                fs
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

/// Benchmarks for glob pattern complexity.
fn bench_glob_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("glob_patterns");
    let fs = create_project_fs();

    // Simple extension glob
    group.bench_function("extension", |b| {
        b.iter(|| fs.list_files_glob(black_box("*.rs")))
    });

    // Recursive glob
    group.bench_function("recursive", |b| {
        b.iter(|| fs.list_files_glob(black_box("**/*.rs")))
    });

    // Directory-specific glob
    group.bench_function("directory", |b| {
        b.iter(|| fs.list_files_glob(black_box("src/module*/*.rs")))
    });

    // Character class glob
    group.bench_function("character_class", |b| {
        b.iter(|| fs.list_files_glob(black_box("src/module[0-9]/*.rs")))
    });

    // Multiple wildcards
    group.bench_function("multi_wildcard", |b| {
        b.iter(|| fs.list_files_glob(black_box("**/module*/**/*.rs")))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_include_simulation,
    bench_exclude_simulation,
    bench_rename_patterns,
    bench_rename_simulation,
    bench_glob_patterns,
);
criterion_main!(benches);
