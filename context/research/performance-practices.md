# Performance Best Practices for Rust Projects

Research findings from examining modern Rust projects and industry best practices for performance optimization, benchmarking, and profiling.

## Benchmarking Frameworks

### Criterion (De Facto Standard)

[Criterion](https://github.com/bheisler/criterion.rs) is the most popular benchmarking harness in the Rust ecosystem:

- Works on both stable and nightly Rust
- Provides statistical analysis with confidence intervals
- Generates HTML reports for visualization
- Detects performance regressions between runs
- Supports parameterized benchmarks

Setup in `Cargo.toml`:
```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "my_benchmark"
harness = false
```

Key practices:
- Use `black_box()` to prevent compiler optimization of benchmark code
- Run multiple times due to system variability
- Warm up the system before measuring

### Divan (Modern Alternative)

[Divan](https://github.com/nvzqz/divan) offers a simpler, more ergonomic approach:

- Uses `#[divan::bench]` attribute (similar simplicity to `#[test]`)
- Tree-structured output reflecting module organization
- Built-in allocation profiling via `AllocProfiler`
- Sample size scaling reduces CI timing noise
- Multi-threaded contention benchmarking support

```rust
#[divan::bench]
fn my_benchmark() {
    // benchmark code
}

fn main() {
    divan::main();
}
```

### Iai / Iai-Callgrind (Instruction Counting)

For CI environments where wall-time is noisy:

- Counts CPU instructions instead of wall-time
- Deterministic results regardless of system load
- Ideal for detecting regressions in CI
- Based on Valgrind's Cachegrind

## Profiling Tools

### CPU Profiling

| Tool | Platform | Description |
|------|----------|-------------|
| [cargo-flamegraph](https://github.com/flamegraph-rs/flamegraph) | Linux, macOS | Wraps perf/DTrace, generates flame graphs |
| [samply](https://github.com/mstange/samply) | Linux, macOS, Windows | Uses Firefox Profiler UI |
| [perf](https://perf.wiki.kernel.org/) | Linux | Sampling-based profiling with call graphs |
| [Instruments](https://developer.apple.com/instruments/) | macOS | Apple's profiling suite |

Best practices:
- Enable frame pointers: `-C force-frame-pointers=yes`
- Enable debug symbols in release: `[profile.release] debug = true`
- Profile on warmed-up, idle machines
- Save baseline measurements before optimization

### Memory Profiling

| Tool | Platform | Description |
|------|----------|-------------|
| [DHAT](https://valgrind.org/docs/manual/dh-manual.html) | Linux | Valgrind tool for heap analysis |
| [dhat-rs](https://docs.rs/dhat) | Cross-platform | Rust implementation, supports heap testing |
| [heaptrack](https://github.com/KDE/heaptrack) | Linux | Tracks all allocations with backtraces |
| [bytehound](https://github.com/koute/bytehound) | Linux | Memory profiler with web UI |
| [Massif](https://valgrind.org/docs/manual/ms-manual.html) | Linux | Valgrind heap profiler |

dhat-rs enables heap usage testing:
```rust
#[test]
fn test_allocations() {
    let _profiler = dhat::Profiler::new_heap();
    // test code
    let stats = dhat::HeapStats::get();
    assert!(stats.total_bytes < 1000);
}
```

## Continuous Benchmarking

### Why It Matters

Performance regressions are like bugs—catch them in CI before production. Key benefits:
- Detect regressions before merge
- Track performance over time
- Establish performance budgets

### Bencher

[Bencher](https://bencher.dev/) is an open-source continuous benchmarking platform:

- Tracks Criterion, Divan, Iai, and custom harnesses
- Statistical threshold-based alerts
- GitHub integration with PR comments
- Historical tracking and visualization

### CI Considerations

Wall-time benchmarks in CI are noisy due to:
- Shared infrastructure variability
- CPU throttling and scheduling
- Background processes

Solutions:
1. Use instruction-counting (Iai) for deterministic results
2. Use statistical thresholds with higher tolerance
3. Run on dedicated hardware (like Rustls does)
4. Focus on relative comparisons, not absolutes

### Rustls Case Study

The [Rustls project](https://github.com/rustls/rustls) exemplifies mature continuous benchmarking:
- Custom benchmarking harness (CI Bench)
- Dedicated bare-metal server
- Dual-mode: instruction count + wall-time
- Delta Interquartile Range statistical thresholds
- Automated PR comments highlighting regressions

## Exemplar Project Strategies

### ripgrep

[ripgrep](https://github.com/BurntSushi/ripgrep) achieves exceptional search performance through:

**Architecture decisions:**
- Parallel directory traversal by default
- SIMD-accelerated literal matching
- Smart I/O: memory-mapped vs. incremental based on context
- Finite automata regex engine with aggressive literal optimization

**Regex optimization:**
- Extracts literal strings from patterns for fast pre-filtering
- Example: `\w+foo\d+` extracts "foo" for candidate line identification
- Falls back to full regex only on candidate lines

**Tunable parameters:**
- `--threads` for CPU core utilization
- `--dfa-size-limit` for complex patterns
- Memory-mapped files for single-file searches
- Incremental reading for large directories

**Build optimization:**
- Supports Profile Guided Optimization (PGO)
- ~10% performance improvement with PGO

### uv (Python Package Manager)

[uv](https://github.com/astral-sh/uv) achieves 10-100x speedup over pip through:

**Layered parallelism:**
- Async I/O via Tokio for network/disk operations
- Rayon thread pool for CPU-intensive work
- Parallel dependency resolution across tree branches

**Optimized metadata fetching:**
- Downloads only wheel metadata, not entire packages
- Uses ZIP Central Directory to fetch specific files

**Efficient caching:**
- Global module cache avoids redundant downloads
- Copy-on-Write and hardlinks minimize disk usage
- Hard links for Python binaries in virtual environments

**Smart dependency resolution:**
- Uses PubGrub solver algorithm
- Conclusively proves satisfiability or unsatisfiability

### Tokio

[Tokio](https://tokio.rs/) demonstrates async runtime optimization:

**Scheduler improvements:**
- Work-stealing scheduler redesign achieved 10x speedups
- 34% throughput increase for Hyper
- 10% improvement for Tonic (gRPC)

**Monitoring:**
- [tokio-metrics](https://github.com/tokio-rs/tokio-metrics) for runtime visibility
- Tracks scheduler behavior, task poll times
- Integrates with Prometheus, Grafana, etc.

**Best practices:**
- Use `BufReader`/`BufWriter` for I/O
- Configure `max_threads` based on workload
- Use Rayon for CPU-bound work, not Tokio
- Be careful with `FuturesUnordered` at scale

## Optimization Techniques

### Zero-Cost Abstractions

Rust's philosophy enables high-level code without runtime overhead:
- Iterators compile to efficient loops
- Closures are often inlined
- Generics monomorphize to specialized code

### Memory Efficiency

- Avoid unnecessary cloning—use references
- Prefer stack allocation over heap when possible
- Use `Box`, `Rc`, `Arc` judiciously
- Consider arena allocators for many small objects
- Use `SmallVec` for small collections

### Compilation Settings

For performance:
```toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
```

### Binary Size Reduction

When size matters more than speed:
```toml
[profile.release]
opt-level = "z"      # Optimize for size
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

Additional techniques:
- Build std from source with `-Z build-std`
- Choose lightweight dependencies
- Consider `no_std` for minimal binaries

## Key Takeaways

1. **Start with Criterion or Divan** for benchmarking—don't rely on ad-hoc timing
2. **Profile before optimizing**—use flamegraphs to find actual bottlenecks
3. **Implement continuous benchmarking** in CI to catch regressions early
4. **Use instruction counting** (Iai) for deterministic CI results
5. **Measure memory too**—allocations impact performance significantly
6. **Learn from exemplars**—ripgrep, uv, and tokio show how to achieve real-world performance
7. **"Good benchmarking is hard"**—but imperfect measurement beats no measurement

## References

### Rust Performance Book
- [Benchmarking](https://nnethercote.github.io/perf-book/benchmarking.html)
- [Profiling](https://nnethercote.github.io/perf-book/profiling.html)
- [Heap Allocations](https://nnethercote.github.io/perf-book/heap-allocations.html)

### Tools
- [Criterion](https://github.com/bheisler/criterion.rs)
- [Divan](https://github.com/nvzqz/divan)
- [Bencher](https://bencher.dev/)
- [cargo-flamegraph](https://github.com/flamegraph-rs/flamegraph)
- [samply](https://github.com/mstange/samply)

### Projects Studied
- [ripgrep](https://github.com/BurntSushi/ripgrep) - [Performance analysis](https://blog.burntsushi.net/ripgrep/)
- [uv](https://github.com/astral-sh/uv)
- [Tokio](https://tokio.rs/) - [Scheduler improvements](https://tokio.rs/blog/2019-10-scheduler)

### Binary Size
- [min-sized-rust](https://github.com/johnthagen/min-sized-rust)
- [Embedded Rust Book - Speed vs Size](https://docs.rust-embedded.org/book/unsorted/speed-vs-size.html)
