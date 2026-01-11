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

// No pattern matching - use methods
if value.is_object() { value.as_object().unwrap() }
if value.is_array() { value.as_array().unwrap() }

// Object.get() needs owned String
obj.get(&key.to_string())  // not obj.get(key)
```

## Key Design Decisions

- **SIMD**: `.cargo/config.toml` sets `-C target-cpu=native` for sonic-rs
- **Exit codes**: Always 0 on success (not 1 for differences)
- **Objects**: Unordered by default (use `-o` for ordered)
- **Arrays**: Ordered by default (use `-s` for set, `-m` for multiset)

## Commit Messages

Commit messages should describe the intention/effect, not summarize the diff. The diff is already visible - explain *why* or *what it achieves*.
