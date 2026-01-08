//! Benchmarks for in-memory filesystem operations.
//!
//! These benchmarks measure the performance of the `MemoryFS` struct,
//! which is central to the common-repo processing pipeline.

use common_repo::filesystem::MemoryFS;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

/// Creates a MemoryFS with a specified number of files.
fn create_fs_with_files(num_files: usize) -> MemoryFS {
    let mut fs = MemoryFS::new();
    for i in 0..num_files {
        let path = format!("src/module{}/file{}.rs", i / 100, i);
        let content = format!("// File {}\nfn main() {{}}\n", i);
        fs.add_file_string(&path, &content).unwrap();
    }
    fs
}

/// Creates a MemoryFS with files in a deep directory structure.
fn create_deep_fs(depth: usize, files_per_level: usize) -> MemoryFS {
    let mut fs = MemoryFS::new();
    let mut file_count = 0;

    fn add_level(fs: &mut MemoryFS, prefix: &str, depth: usize, files: usize, count: &mut usize) {
        for i in 0..files {
            let path = format!("{}/file{}.rs", prefix, i);
            let content = format!("// File {}\n", count);
            fs.add_file_string(&path, &content).unwrap();
            *count += 1;
        }
        if depth > 0 {
            for i in 0..3 {
                let new_prefix = format!("{}/level{}", prefix, i);
                add_level(fs, &new_prefix, depth - 1, files, count);
            }
        }
    }

    add_level(&mut fs, "src", depth, files_per_level, &mut file_count);
    fs
}

fn bench_add_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("fs_add_file");

    // Benchmark adding files to empty filesystem
    group.bench_function("single_file", |b| {
        b.iter(|| {
            let mut fs = MemoryFS::new();
            fs.add_file_string(black_box("test.rs"), black_box("content"))
                .unwrap();
            fs
        })
    });

    // Benchmark adding many files
    for count in [10, 100, 500] {
        group.bench_with_input(BenchmarkId::new("batch", count), &count, |b, &count| {
            b.iter(|| {
                let mut fs = MemoryFS::new();
                for i in 0..count {
                    fs.add_file_string(format!("file{}.rs", i), "content")
                        .unwrap();
                }
                fs
            })
        });
    }

    group.finish();
}

fn bench_get_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("fs_get_file");

    for size in [100, 500, 1000] {
        let fs = create_fs_with_files(size);

        group.bench_with_input(BenchmarkId::new("lookup", size), &fs, |b, fs| {
            b.iter(|| {
                // Look up file in the middle of the range
                fs.get_file(black_box(&format!(
                    "src/module{}/file{}.rs",
                    size / 200,
                    size / 2
                )))
            })
        });
    }

    group.finish();
}

fn bench_list_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("fs_list_files");

    for size in [100, 500, 1000] {
        let fs = create_fs_with_files(size);

        group.bench_with_input(BenchmarkId::new("all", size), &fs, |b, fs| {
            b.iter(|| fs.list_files())
        });
    }

    group.finish();
}

fn bench_list_files_glob(c: &mut Criterion) {
    let mut group = c.benchmark_group("fs_list_files_glob");

    let fs = create_fs_with_files(1000);

    // Simple glob
    group.bench_function("simple_glob", |b| {
        b.iter(|| fs.list_files_glob(black_box("*.rs")))
    });

    // Wildcard glob
    group.bench_function("wildcard_glob", |b| {
        b.iter(|| fs.list_files_glob(black_box("**/*.rs")))
    });

    // Specific path glob
    group.bench_function("specific_glob", |b| {
        b.iter(|| fs.list_files_glob(black_box("src/module5/*.rs")))
    });

    // Complex glob
    group.bench_function("complex_glob", |b| {
        b.iter(|| fs.list_files_glob(black_box("src/module[0-9]/**/*.rs")))
    });

    group.finish();
}

fn bench_merge(c: &mut Criterion) {
    let mut group = c.benchmark_group("fs_merge");

    for size in [100, 500, 1000] {
        let fs1 = create_fs_with_files(size);
        let fs2 = create_fs_with_files(size);

        group.bench_with_input(
            BenchmarkId::new("disjoint", size),
            &(fs1, fs2),
            |b, (f1, f2)| {
                b.iter(|| {
                    let mut merged = f1.clone();
                    merged.merge(black_box(f2));
                    merged
                })
            },
        );
    }

    // Benchmark merge with overlapping files
    for size in [100, 500] {
        let mut fs1 = MemoryFS::new();
        let mut fs2 = MemoryFS::new();
        for i in 0..size {
            fs1.add_file_string(format!("file{}.rs", i), "content1")
                .unwrap();
            fs2.add_file_string(format!("file{}.rs", i), "content2")
                .unwrap();
        }

        group.bench_with_input(
            BenchmarkId::new("overlapping", size),
            &(fs1, fs2),
            |b, (f1, f2)| {
                b.iter(|| {
                    let mut merged = f1.clone();
                    merged.merge(black_box(f2));
                    merged
                })
            },
        );
    }

    group.finish();
}

fn bench_rename_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("fs_rename");

    for size in [100, 500, 1000] {
        let fs = create_fs_with_files(size);

        group.bench_with_input(BenchmarkId::new("single", size), &fs, |b, fs| {
            b.iter(|| {
                let mut fs_clone = fs.clone();
                fs_clone
                    .rename_file(
                        black_box(&format!("src/module{}/file{}.rs", size / 200, size / 2)),
                        black_box("renamed.rs"),
                    )
                    .unwrap();
                fs_clone
            })
        });
    }

    group.finish();
}

fn bench_copy_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("fs_copy");

    for size in [100, 500, 1000] {
        let fs = create_fs_with_files(size);

        group.bench_with_input(BenchmarkId::new("single", size), &fs, |b, fs| {
            b.iter(|| {
                let mut fs_clone = fs.clone();
                fs_clone
                    .copy_file(
                        black_box(&format!("src/module{}/file{}.rs", size / 200, size / 2)),
                        black_box("copied.rs"),
                    )
                    .unwrap();
                fs_clone
            })
        });
    }

    group.finish();
}

fn bench_exists(c: &mut Criterion) {
    let mut group = c.benchmark_group("fs_exists");

    for size in [100, 500, 1000] {
        let fs = create_fs_with_files(size);

        group.bench_with_input(BenchmarkId::new("existing", size), &fs, |b, fs| {
            b.iter(|| {
                fs.exists(black_box(&format!(
                    "src/module{}/file{}.rs",
                    size / 200,
                    size / 2
                )))
            })
        });

        group.bench_with_input(BenchmarkId::new("nonexisting", size), &fs, |b, fs| {
            b.iter(|| fs.exists(black_box("nonexistent/path/file.rs")))
        });
    }

    group.finish();
}

fn bench_deep_structure(c: &mut Criterion) {
    let mut group = c.benchmark_group("fs_deep_structure");

    // Create filesystems with different depths
    for depth in [2, 3, 4] {
        let fs = create_deep_fs(depth, 5);
        let file_count = fs.len();

        group.bench_with_input(
            BenchmarkId::new("list_glob", format!("depth{}_files{}", depth, file_count)),
            &fs,
            |b, fs| b.iter(|| fs.list_files_glob(black_box("**/*.rs"))),
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_add_file,
    bench_get_file,
    bench_list_files,
    bench_list_files_glob,
    bench_merge,
    bench_rename_file,
    bench_copy_file,
    bench_exists,
    bench_deep_structure,
);
criterion_main!(benches);
