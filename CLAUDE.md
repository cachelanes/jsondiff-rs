# CLAUDE.md

Guidance for AI assistants working on this codebase.

## Build & Run

```bash
cargo build --release
cargo run -- file1.json file2.json
```

## Architecture

```
src/
├── main.rs              # Entry point
├── cli/args.rs          # Clap CLI definitions
├── parser/json.rs       # sonic-rs JSON parsing
├── diff/
│   ├── types.rs         # DiffOp, JsonPath, DiffConfig
│   ├── engine.rs        # Main diff logic
│   ├── object.rs        # Object comparison (unordered default)
│   └── array.rs         # Array comparison (ordered/set/multiset)
├── output/formatter.rs  # Pretty/JSON/summary output
└── error.rs             # Error types
```

## sonic-rs Gotchas

sonic-rs is not serde_json. Key differences:

```rust
// Must import traits for API access
use sonic_rs::{JsonValueTrait, JsonContainerTrait};

// No pattern matching on Value - use if-let with as_*() methods
if let Some(obj) = value.as_object() { /* use obj */ }
if let (Some(l), Some(r)) = (left.as_array(), right.as_array()) { /* use l, r */ }

// Object.get() needs owned String
obj.get(&key.to_string())  // not obj.get(key)
```

## Key Design Decisions

- **SIMD**: `.cargo/config.toml` sets `-C target-cpu=native` for sonic-rs
- **Exit codes**: Always 0 on success (not 1 for differences)
- **Objects**: Unordered by default (use `-o` for ordered)
- **Arrays**: Ordered by default (use `-s` for set, `-m` for multiset)

## Code Conventions

- **Prefer `if let` over `is_*()` + `unwrap()`**: Use `if let Some(x) = value.as_*()` instead of `if value.is_*() { value.as_*().unwrap() }`. This eliminates the redundant type check and the unwrap in one go.
- **Readability over lint compliance**: If a clippy lint suggests a less readable transformation (e.g. `option_if_let_else` wanting `map_or_else` over a clear `match`), prefer the readable form and add a targeted `#[expect]` with a reason. If these pile up, disable the lint in `Cargo.toml` instead.
- **`.expect()` messages should state the expectation**: Write what you expect to be true, not what you're doing. e.g. `"results should have a second element"` not `"parallel parse of 2 files"`.
- **Unused trait imports**: Import sonic-rs traits as `_ ` to avoid unused-name warnings: `use sonic_rs::JsonValueTrait as _`.

## Commit Messages

Commit messages should describe the intention/effect, not summarize the diff. The diff is already visible - explain *why* or *what it achieves*.
