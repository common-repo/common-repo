# Rust TDD

## Workflow

Work in vertical slices. ONE test, ONE implementation, repeat. Never batch.

### Phase 1 — Compiler Red

Write one test for one behavior. Write type signatures and `todo!()` stubs for anything new. Run `cargo check`. Fix every compiler error. Do not implement logic yet.

### Phase 2 — Runtime Red

Run `cargo test <test_name>`. The test must compile and fail — via `todo!()` panic or wrong return value. If it passes, the test is not asserting anything new; rewrite it.

### Phase 3 — Green

Write minimum code to pass this one test. Nothing speculative. Run `cargo test`. All tests pass.

### Phase 4 — Refactor

Restructure freely. Run `cargo test` after each change. Do not add behavior. Confirm all tests still green before next cycle.

Repeat from Phase 1.

## Rules

- NEVER write implementation before a failing test exists.
- NEVER write more than one test before implementing.
- NEVER modify existing tests to make them pass — fix the code.
- Write each test from the REQUIREMENT, not from knowledge of the planned implementation. Use literal expected values, not computed ones.
- If a compiler error takes more than 2 fix attempts, reconsider the design.

### Rust-Specific Rules

- **Types before bodies.** Define signatures first, fill bodies with `todo!()`. Let `cargo check` validate your design before writing logic.
- **Derive eagerly.** Add `#[derive(Debug, Clone, PartialEq, Eq)]` to every type under test when you define it — including all nested types.
- **Owned types in tests.** Use `String` not `&str`, `Vec<String>` not `Vec<&str>`, `PathBuf` not `&Path`. Clone freely. Match the exact owned type the function returns.
- **Trait-based DI, not mocks.** Define traits for external boundaries. Write simple in-memory implementations. Reserve `mockall` only for call-count verification.
- **Async tests.** Use `#[tokio::test]`, never manual `Runtime::new()`. Use `tokio::sync::Mutex` (not `std`) when holding guards across `.await`. Use `Arc` not `Rc` in async code.
- **Do not test compiler guarantees.** Null safety, type mismatches, exhaustive matching, data races — `rustc` handles these.

---

## Failure Mode Reference

### FM1: Implementation-Biased Tests / Context Pollution

The test writer designs tests around the implementation they are planning, or computes expected values using the same logic as the implementation. Tests pass by construction.

**Prevention**: Describe the requirement first. Write a test with literal expected values. Do not plan the implementation until the test is written and failing.

**WRONG:**
```rust
let discount = price * 0.1; // duplicates the implementation formula
assert_eq!(calculate_discount(price), discount);
```

**RIGHT:**
```rust
assert_eq!(calculate_discount(100.0), 10.0);
```

### FM2: Borrow Checker Cascades (E0502 / E0499)

**WRONG:**
```rust
#[test]
fn test_update_entry() {
    let mut map = HashMap::new();
    map.insert("key", vec![1, 2, 3]);
    let values = map.get("key").unwrap(); // immutable borrow
    map.insert("key", vec![4, 5, 6]);     // E0502: mutable borrow
    assert_eq!(values, &vec![1, 2, 3]);
}
```

**RIGHT:**
```rust
#[test]
fn test_update_entry() {
    let mut map = HashMap::new();
    map.insert("key", vec![1, 2, 3]);
    let values = map.get("key").unwrap().clone(); // clone breaks the borrow
    map.insert("key", vec![4, 5, 6]);
    assert_eq!(values, vec![1, 2, 3]);
}
```

### FM3: Use-After-Move (E0382)

LLMs reuse variables after passing them by value.

**WRONG:**
```rust
#[test]
fn test_process_and_display() {
    let data = build_test_data();
    let result = process(data);        // data moved here
    assert_eq!(format!("{:?}", data),  // E0382: use after move
               "TestData { ... }");
    assert!(result.is_ok());
}
```

**RIGHT:**
```rust
#[test]
fn test_process_and_display() {
    let data = build_test_data();
    let display = format!("{:?}", data); // use before move
    let result = process(data);
    assert_eq!(display, "TestData { ... }");
    assert!(result.is_ok());
}
```

Or clone before the consuming call: `let result = process(data.clone());`

### FM4: Over-Mocking

**WRONG:**
```rust
let mut mock = MockUserStore::new();
mock.expect_find_by_id().with(eq(42)).returning(|_| Ok(Some(user)));
```

**RIGHT:**
```rust
struct FakeUserStore { users: HashMap<u64, User> }
impl UserStore for FakeUserStore {
    fn find_by_id(&self, id: u64) -> Result<Option<User>, StoreError> {
        Ok(self.users.get(&id).cloned())
    }
}
```

### FM5: Missing Derives

Add `#[derive(Debug, Clone, PartialEq, Eq)]` when you define the type — including all nested types. If a field from an external crate lacks these traits, compare individual fields instead.

### FM6: Async Runtime Issues

**WRONG:**
```rust
#[test]
fn test_fetch() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async { /* panics if code internally creates a runtime */ });
}
```

**RIGHT:**
```rust
#[tokio::test]
async fn test_fetch() {
    let result = fetch_data().await;
    assert!(result.is_ok());
}
```

Ensure `tokio = { version = "1", features = ["macros", "rt-multi-thread"] }` in `[dev-dependencies]`.

### FM7: Iterative Compiler Appeasement

The cascade: add `.clone()` → type needs `Clone` → field is `!Clone` → wrap in `Arc` → needs `Send + Sync` → 40 lines of boilerplate testing nothing.

**Rule**: If a fix creates a new error, stop. Ask: Can I test through a simpler interface? Am I holding a reference I don't need? Should I use owned values?

---

## Reference: The todo!() Stub Pattern

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    entries: HashMap<String, String>,
}

impl Config {
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        todo!()
    }
    pub fn get(&self, key: &str) -> Option<&str> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_key_value() {
        let config = Config::parse("name = Alice").unwrap();
        assert_eq!(config.get("name"), Some("Alice"));
    }
}
```

`cargo check` passes (compiler green). `cargo test` panics on `todo!()` (runtime red). Implement `parse` to reach green.

## Reference: Trait-Based DI

```rust
pub trait UserStore {
    fn find_by_id(&self, id: u64) -> Result<Option<User>, StoreError>;
    fn save(&self, user: &User) -> Result<(), StoreError>;
}

#[cfg(test)]
struct FakeUserStore {
    users: RefCell<HashMap<u64, User>>,
}

#[cfg(test)]
impl UserStore for FakeUserStore {
    fn find_by_id(&self, id: u64) -> Result<Option<User>, StoreError> {
        Ok(self.users.borrow().get(&id).cloned())
    }
    fn save(&self, user: &User) -> Result<(), StoreError> {
        self.users.borrow_mut().insert(user.id, user.clone());
        Ok(())
    }
}
```

Use generics (`fn process<S: UserStore>(store: &S)`) for zero-cost dispatch. Keep trait methods non-generic to preserve object safety.

## Reference: Testing Pyramid

| Layer | Location | Access | Speed | Use For |
|---|---|---|---|---|
| Unit | `#[cfg(test)] mod tests` in source files | Private + public | Fast | Logic, algorithms, edge cases |
| Integration | `tests/*.rs` | Public API only | Medium | API contracts, CLI behavior |
| Doc tests | `///` comments on public items | Public API only | Slow | API examples that stay correct |

```bash
cargo check                  # Phase 1: compiler errors (fastest)
cargo test test_name         # Phase 2-3: single test
cargo test                   # Full suite before commit
cargo nextest run            # Parallel execution (large suites)
```

## Reference: Recommended Crates

| Crate | Purpose | When to Add |
|---|---|---|
| `pretty_assertions` | Colored diffs in `assert_eq!` | Always |
| `rstest` | Parameterized tests, fixtures | Table-driven tests |
| `proptest` | Property-based testing | Algorithms, invariants |
| `insta` | Snapshot testing | Complex output formats |
| `cargo-nextest` | Parallel test runner | Suites > 30 seconds |
| `tempfile` | Temporary files/dirs | Tests touching filesystem |
| `assert_cmd` | CLI binary testing | CLI applications |
| `wiremock` | HTTP mock server | HTTP client tests |

## Reference: What the Compiler Tests For You

Do not write tests for these:

| Guarantee | Mechanism |
|---|---|
| No null dereferences | `Option<T>` |
| No data races | Ownership + `Send`/`Sync` |
| No use-after-free | Borrow checker |
| No unhandled variants | Exhaustive `match` |
| No type mismatches | Static typing |
