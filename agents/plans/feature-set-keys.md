# Set Key Feature

Match array objects by a specific key field instead of by position.

## Design Decisions Summary

### 1. CLI Argument Syntax: `--set-key "path.key"`

| Decision | `--set-key "users.id"` |
|----------|------------------------|
| Alternatives considered | `--set-keys "users:id"`, `--set-key-path "users.id"` |
| Reasoning | Dot notation is familiar (jq, gron, JS). Single separator is simpler than `path:key`. Name `--set-key` is clearer than `--set-key-path` since the value includes both path and key. |

**Composite keys:** Multiple arguments with same path prefix are grouped.
```bash
--set-key "users.id" --set-key "users.type"  # Match by [id, type]
```

### 2. Output Path Notation: `[id=1]`

| Decision | `$.users[id=1].name` |
|----------|----------------------|
| Alternatives considered | `[@id=1]` (with `@` prefix for visual distinction) |
| Reasoning | The `=` already distinguishes from index access (`[0]`). Adding `@` creates unnecessary clutter. Other tools (jd) also use plain notation. |

### 3. No Array Wildcards in Input

| Decision | No `[*]` or `[]` needed in `--set-key` paths |
|----------|----------------------------------------------|
| Reasoning | The argument specifies *which arrays* get key-matching, not a query. Every set-key path inherently targets an array, so wildcards would be redundant clutter. Simple dot-path suffices. |

**Root array:** No dot needed: `--set-key "id"` for `$[*].id` (simplest case = simplest syntax)

### 4. Missing Keys: Lenient Default with Strict Flag

| Decision | `--set-key-allow-missing` defaults to `true` in CLI |
|----------|-----------------------------------------------------|
| Alternatives considered | Fail-fast by default, stderr warnings |
| Reasoning | CLI output is self-explanatory with `[{}]` marker for fallback elements. Stderr warnings fail silently in pipes. Library defaults to strict (`false`) for programmatic use. |

### 5. Duplicate Keys: Lenient Default with Strict Flag

| Decision | `--set-key-allow-duplicates` defaults to `true` in CLI |
|----------|--------------------------------------------------------|
| Alternatives considered | Stderr warning, error by default |
| Reasoning | Same as missing keys—stderr is problematic in pipes/scripts. First-match-wins is industry standard (jd, jsondiffpatch, zjsonpatch). Strict mode available for CI. |

### 6. Flags with Defaults (not Strict Flags)

| Decision | `--set-key-allow-missing=true` (not `--set-key-strict`) |
|----------|--------------------------------------------------------|
| Reasoning | Allows changing defaults later based on user feedback without breaking API. `--no-set-key-allow-missing` for strict mode. |

---

## CLI Interface

```bash
# Single key
jsondiff --set-key "users.id" file1.json file2.json

# Composite key (same path, multiple keys)
jsondiff --set-key "users.id" --set-key "users.type" file1.json file2.json

# Multiple paths
jsondiff --set-key "users.id" --set-key "orders.order_id" file1.json file2.json

# Root array (no dot needed)
jsondiff --set-key "id" file1.json file2.json

# Nested path
jsondiff --set-key "data.items.sku" file1.json file2.json

# Strict mode (for CI/scripts)
jsondiff --set-key "users.id" --no-set-key-allow-missing --no-set-key-allow-duplicates f1.json f2.json
```

## jd Equivalence

```bash
# jd (global setkeys):
jd -set -setkeys id a.json b.json

# jsondiff (per-path):
jsondiff --set-key "users.id" a.json b.json
```

## Example

```json
// left.json
{"users": [{"id": 1, "name": "Alice"}, {"id": 2, "name": "Bob"}]}

// right.json
{"users": [{"id": 2, "name": "Robert"}, {"id": 1, "name": "Alice"}]}
```

**Without `--set-key`:** Shows all fields changed (position mismatch)

**With `--set-key "users.id"`:** Only shows `$.users[id=2].name` changed

```
$.users[id=2].name
- "Bob"
+ "Robert"
```

## Output Path Notation

| Input Path | Output Path |
|------------|-------------|
| Single key | `$.users[id=1].name` |
| Composite | `$.items[name=foo,type=bar].price` |
| Special chars | `$.users[id="value,with,commas"]` |

## Edge Cases

| Flag | CLI Default | Library Default | When `false` |
|------|-------------|-----------------|--------------|
| `--set-key-allow-missing` | `true` | `false` | Error on missing key field |
| `--set-key-allow-duplicates` | `true` | `false` | Error on duplicate keys |

| Case | Behavior |
|------|----------|
| Null key value | Valid key `"null"` |
| Non-object in array | Same as missing key |

### Strict Mode Errors

```
error: object at $.users[2] missing required key field "id"
hint: use --set-key-allow-missing to fall back to set comparison
```

```
error: duplicate key [id=1] found at $.users[0] and $.users[3]
hint: use --set-key-allow-duplicates to use first occurrence
```

### Fallback Output (lenient mode)

```
$.users[{}]
- {"type": "metadata"}
+ {"type": "footer"}
```

---

## Implementation

### Files to Modify

| File | Changes |
|------|---------|
| `src/diff/types.rs` | Add `KeyMatch` PathSegment, `SetKeyConfig` struct |
| `src/diff/array.rs` | New `diff_arrays_with_set_key()` function |
| `src/diff/engine.rs` | Dispatch to key-based comparison when path matches |
| `src/cli/args.rs` | Add `--set-key`, `--set-key-allow-missing`, `--set-key-allow-duplicates` |
| `src/main.rs` | Parse and group set-key args into config |

### New Types

```rust
// PathSegment variant
KeyMatch(Vec<(String, String)>),  // [(key_name, key_value), ...]

// Configuration
pub struct SetKeyConfig {
    pub paths: HashMap<String, Vec<String>>,  // "users" -> ["id", "type"]
    pub allow_missing: bool,
    pub allow_duplicates: bool,
}
```

### Algorithm

1. **Parse CLI:**
   - No `.` → root array: `"id"` → `{"" => ["id"]}`
   - Has `.` → split on last `.`: `"users.id"` → `{"users" => ["id"]}`
   - Group same paths: `"users.id"`, `"users.type"` → `{"users" => ["id", "type"]}`
2. **At each array:** Check if current path (e.g., `$.users`) matches config
3. **If matched:**
   - Partition into keyed (have all keys) and unkeyed elements
   - Validate based on `allow_missing` / `allow_duplicates`
   - Build key→object maps, match pairs, recurse diff
   - Report additions/removals, compare unkeyed via set semantics
4. **If not matched:** Use default `array_mode`

---

## Research Notes

### Other Tools Surveyed

| Tool | Syntax | Notes |
|------|--------|-------|
| [jd](https://github.com/josephburnett/jd) | `-setkeys id` (global) or `-opts='[{"@":["path"],"^":[{"setkeys":["id"]}]}]'` | Complex JSON syntax for per-path |
| [jq](https://jqlang.org/manual/) | `.[]` iterator, `.foo.bar` paths | No set-key concept |
| [gron](https://github.com/tomnomnom/gron) | `json.foo[0].bar` (JS-style) | Inspired our dot notation |
| [JSONPath RFC 9535](https://www.rfc-editor.org/rfc/rfc9535.html) | `$`, `[*]`, `$.foo.bar[0]` | We don't need full JSONPath parsing |

### Duplicate Key Handling in Other Tools

All use first-match-wins: jd, jsondiffpatch, zjsonpatch, DeepDiff.

---

## Rejected Alternatives

### 1. Bracket Notation: `--set-key "users[id]"` or `--set-key "users[id,name]"`

```bash
# Rejected syntax
--set-key "users[id]"
--set-key "users[id,name]"    # Composite in one arg
--set-key "[id]"              # Root array
```

**Pros:** Mirrors output format (`$.users[id=1]`), composite keys in single argument.

**Why rejected:** Users would likely add brackets for intermediate arrays (`data[].items[id]` instead of `data.items.id`), causing confusion. The discovery cost outweighs the benefit—users type the command before seeing output, so familiar dot notation wins.

### 2. Output Path with `@` Prefix: `[@id=1]`

**Why rejected:** The `=` already distinguishes key-match from index access (`[0]`). Adding `@` creates visual clutter without meaningful benefit. Considered for "visual distinction at a glance" but rejected as unnecessary.

### 3. Global Set-Keys (like jd): `--set-key id`

```bash
# jd style - applies to ALL arrays
jd -setkeys id a.json b.json
```

**Why rejected:** Real-world JSON has different ID fields per array (`users.id`, `orders.order_id`, `products.sku`). Global application only works by accident. Per-path is explicit and safer.

### 4. Colon Separator: `--set-keys "users:id,name"`

```bash
# Rejected syntax
--set-keys "users:id"
--set-keys "users:id,name"    # Composite
```

**Why rejected:** Two separators (`:` for path/key, `,` for multiple keys) is more complex than single dot separator. Dot notation is already familiar from jq/gron/JS.

### 5. Leading Dot for Root Array: `--set-key ".id"`

**Why rejected:** Most common use case (root array by single key) should have simplest syntax. `--set-key "id"` is cleaner than `--set-key ".id"`.

### 6. Fail-Fast by Default for Missing Keys

**Why rejected:** CLI users want to see output, not errors. The `[{}]` marker in output makes fallback behavior self-explanatory. Library can default to strict; CLI defaults to lenient.

### 7. Stderr Warnings for Duplicate Keys

**Why rejected:** Warnings to stderr fail silently when output is piped. Either error (strict mode) or silently handle (lenient mode)—no middle ground that works reliably in scripts.

### 8. Single `--set-key-strict` Flag

```bash
# Rejected
--set-key "users.id" --set-key-strict
```

**Why rejected:** Less granular than separate `--set-key-allow-missing` and `--set-key-allow-duplicates`. Also, using allow-flags with defaults lets us change defaults later without breaking the API.

### 9. Supporting Both Dot and Bracket Syntax

**Why rejected:** "Two ways to do the same thing" adds cognitive overhead for users and documentation complexity. Better to pick one and commit.

### 10. Array Wildcards in Input Paths: `--set-key "data[*].items.id"`

**Why rejected:** We're specifying *which arrays* get key-matching, not querying JSON. Every set-key path inherently targets an array, so `[*]` would be redundant clutter on every path.
