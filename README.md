# jsondiff

A lightning-fast JSON diff tool with beautiful, colorful output.

## Features

- **Fast**: SIMD-accelerated JSON parsing via [sonic-rs](https://github.com/cloudwego/sonic-rs) (3-6x faster than serde_json)
- **Beautiful**: Colorful unified-diff-style output with JSON path notation
- **Flexible array handling**:
  - Ordered comparison (default)
  - Set mode (`-s`): ignore order and duplicates
  - Multiset mode (`-m`): ignore order, count duplicates
- **Smart object comparison**: Unordered by default (semantic matching)
- **Multiple output formats**: Pretty, JSON, or summary
- **Stdin support**: Pipe JSON directly
- **Parallel file loading**: Uses rayon for concurrent I/O
- **Memory-efficient**: Memory-maps large files (>10MB)

## Installation

```bash
cargo install --path .
```

Or build from source:

```bash
cargo build --release
./target/release/jsondiff --help
```

## Usage

### Basic comparison

```bash
jsondiff old.json new.json
```

Output:
```
$.config.database
- "postgresql://localhost:5432/mydb"
+ "postgresql://prod-server:5432/mydb"

$.users[0].email
- "john@old-email.com"
+ "john@new-email.com"

----------------------------------------
0 added, 0 removed, 2 modified
```

### Array comparison modes

Given two files with the same users in different order:
```json
// left.json
{
  "users": [
    {"id": 1, "name": "Alice"},
    {"id": 2, "name": "Bob"}
  ]
}

// right.json
{
  "users": [
    {"id": 2, "name": "Bob"},
    {"id": 1, "name": "Alice"}
  ]
}
```

**Ordered mode (default)** — position matters:
```bash
$ jsondiff left.json right.json
$.users[0].id
- 1
+ 2

$.users[0].name
- "Alice"
+ "Bob"

$.users[1].id
- 2
+ 1

$.users[1].name
- "Bob"
+ "Alice"

----------------------------------------
0 added, 0 removed, 4 modified
```

**Set mode (`-s`)** — ignore order, same elements = no diff:
```bash
$ jsondiff -s left.json right.json
No differences found.
```

| Mode | Flag | Order matters? | Duplicates matter? |
|------|------|----------------|-------------------|
| Ordered | (default) | Yes | Yes |
| Set | `-s` | No | No |
| Multiset | `-m` | No | Yes |

### Output formats

```bash
# Pretty output (default)
jsondiff file1.json file2.json

# JSON output (machine-readable)
jsondiff -f json file1.json file2.json

# Summary only
jsondiff -f summary file1.json file2.json
```

### Stdin support

```bash
cat file1.json | jsondiff - file2.json
curl https://api.example.com/data | jsondiff - expected.json
```

### Other options

```bash
# Disable colors
jsondiff --no-color file1.json file2.json

# Compact output (no summary)
jsondiff -c file1.json file2.json

# Compare objects as ordered (key order matters)
jsondiff -o file1.json file2.json
```

## CLI Reference

```
jsondiff [OPTIONS] <FILE1> <FILE2>

Arguments:
  <FILE1>  First JSON file to compare (use "-" for stdin)
  <FILE2>  Second JSON file to compare

Options:
  -s, --array-set         Treat arrays as unordered sets
  -m, --array-multiset    Treat arrays as unordered multisets
  -o, --ordered-objects   Compare objects as ordered (key order matters)
  -f, --format <FORMAT>   Output format [default: pretty] [values: pretty, json, summary]
      --no-color          Disable colored output
  -c, --compact           Show only paths that differ
  -h, --help              Print help
  -V, --version           Print version
```

## Exit Codes

- `0`: Success (regardless of whether differences were found)
- `2`: File not found
- `3`: Invalid JSON
- `4`: File read error

## Output Ordering

Diff output is deterministic and follows the input JSON structure:
- Removed and modified keys appear in the order they occur in the left (original) file
- Added keys appear in the order they occur in the right (new) file

This makes output predictable and easy to correlate with source files.

## Performance

Benchmark results on a typical development machine (run `cargo bench` to reproduce):

### Object Comparison

| Size | Identical | 10% Different | 50% Different | 100% Different |
|------|-----------|---------------|---------------|----------------|
| 10 keys | 0.73 µs | 2.6 µs | 3.1 µs | 3.9 µs |
| 100 keys | 46 µs | 41 µs | 52 µs | 35 µs |
| 1000 keys | 4.4 ms | 2.3 ms | 2.5 ms | 359 µs |

### Array Comparison

| Size | Ordered (Myers) | Set Mode | Multiset Mode |
|------|-----------------|----------|---------------|
| 10 elements | 2.6 µs | 25 µs | 28 µs |
| 100 elements | 86 µs | 2.1 ms | 117 µs |
| 500 elements | 937 µs | 53 ms | 502 µs |
| 1000 elements | 2.9 ms | 204 ms | 982 µs |

### Nested Structures

| Depth | Time |
|-------|------|
| 2 levels | 3.9 µs |
| 4 levels | 59 µs |
| 6 levels | 672 µs |

**Notes:**
- Ordered array comparison uses the Myers diff algorithm, which is O(n*d) where d is the edit distance
- Set mode has O(n²) complexity for deep equality checks on each element pair
- Multiset mode uses hash-based counting, making it faster than set mode for large arrays
- Object comparison is O(n) for key iteration plus recursive comparison of values

### Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark group
cargo bench -- diff_objects
cargo bench -- diff_arrays
cargo bench -- diff_nested

# Memory profiling (use external tools)
heaptrack ./target/release/jsondiff file1.json file2.json
valgrind --tool=dhat ./target/release/jsondiff file1.json file2.json
```

## Future Features

The following features are planned but not yet implemented:

- `--set-keys <KEY>`: Match array objects by a specific property
- `--ignore <PATH>`: Exclude paths from comparison
- JSON Patch (RFC 6902) output format

