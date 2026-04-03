---
name: rust-dev
description: |
  Rust development patterns and guidelines. Activates when:
  - Writing or reviewing Rust code (.rs files)
  - Discussing ownership, lifetimes, or borrowing
  - Designing APIs or module structure
  - Choosing error handling or async patterns
---

# Rust Development Guidelines

## Microsoft's AI Guidelines for Rust

Key principles for AI-assisted Rust development:

1. **Prefer explicit over implicit** - Use explicit type annotations, especially for function signatures and struct fields
2. **Embrace the type system** - Let the compiler catch errors; don't fight the borrow checker
3. **Document invariants** - Comments should explain why, not what
4. **Small, focused functions** - Easier for AI to understand and modify
5. **Consistent naming** - Follow Rust naming conventions (snake_case for functions, CamelCase for types)
6. **Test-driven clarity** - Tests serve as executable documentation

## Project Structure

Standard Cargo layout:
```
src/
├── lib.rs          # Library root (if library)
├── main.rs         # Binary root (if binary)
├── bin/            # Additional binaries
└── module/
    ├── mod.rs      # Module root
    └── submodule.rs
tests/              # Integration tests
benches/            # Benchmarks
examples/           # Example programs
```

## Idiomatic Patterns

### Builder Pattern
Use for complex object construction with many optional fields:
```rust
Server::builder()
    .port(8080)
    .threads(4)
    .build()?
```

### Newtype Pattern
Wrap primitives for type safety and domain modeling:
```rust
struct UserId(u64);
struct Email(String);
```

### RAII (Resource Acquisition Is Initialization)
Let destructors handle cleanup:
```rust
let _guard = lock.lock();
// Lock automatically released when _guard drops
```

### Typestate Pattern
Encode state machines in the type system:
```rust
struct Connection<S: State> { ... }
impl Connection<Disconnected> {
    fn connect(self) -> Connection<Connected> { ... }
}
```

### Extension Traits
Add methods to external types:
```rust
trait StringExt {
    fn is_blank(&self) -> bool;
}
impl StringExt for str { ... }
```

## Error Handling

### Decision Tree

**Use `thiserror` when:**
- Writing a library
- Errors need to be matched on by callers
- You want structured error types

```rust
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("missing field: {0}")]
    MissingField(String),
    #[error("invalid value for {field}: {value}")]
    InvalidValue { field: String, value: String },
}
```

**Use `anyhow` when:**
- Writing a binary/application
- Errors just need to be reported, not handled
- You want easy error context

```rust
use anyhow::{Context, Result};

fn load_config() -> Result<Config> {
    let contents = fs::read_to_string("config.toml")
        .context("failed to read config file")?;
    toml::from_str(&contents)
        .context("failed to parse config")
}
```

### The `?` Operator
Use `?` for propagation, `.context()` or `.map_err()` to add information.

## Testing Patterns

### Unit Tests
Place in the same file with `#[cfg(test)]`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() { ... }
}
```

### Integration Tests
Place in `tests/` directory, each file is a separate crate.

### Property-Based Testing
Use `proptest` or `quickcheck` for generative testing:
```rust
proptest! {
    #[test]
    fn parse_roundtrip(s: String) {
        let parsed = parse(&s)?;
        assert_eq!(format(&parsed), s);
    }
}
```

### Test Organization
- Use descriptive test names: `test_empty_input_returns_none`
- Group related tests in submodules
- Use `#[ignore]` for slow tests, run with `cargo test -- --ignored`

## Lifetime Guidelines

1. **Elision first** - Let the compiler infer when possible
2. **Named lifetimes** - Use descriptive names: `'input`, `'conn`, not just `'a`
3. **Avoid `'static`** - Unless truly needed (const data, thread spawning)
4. **Owned vs borrowed** - Prefer owned types in structs unless you have a specific reason

## Async Patterns

When using async (tokio runtime):
- Use `async fn` at API boundaries
- Prefer `tokio::spawn` for independent tasks
- Use `tokio::select!` for racing futures
- Avoid holding locks across `.await` points

## Resources

Essential references for Rust development:

- [Rust Design Patterns Book](https://rust-unofficial.github.io/patterns/) - Comprehensive pattern catalog
- [Microsoft Pragmatic Rust Guidelines](https://microsoft.github.io/rust-guidelines/) - Production Rust advice
- [Microsoft AI Guidelines for Rust](https://microsoft.github.io/rust-guidelines/guidelines/ai/) - AI-assisted development
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) - API design checklist
- [GoF Patterns in Rust](https://github.com/fadeevab/design-patterns-rust) - Classic patterns, Rust idioms
- [Cargo Project Layout](https://doc.rust-lang.org/cargo/guide/project-layout.html) - Official structure docs
- [rust-skills Repository](https://github.com/ZhangHanDong/rust-skills) - Practical Rust techniques
