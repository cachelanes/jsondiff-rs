# Set Keys Feature

Match array objects by a specific key field instead of by position.

## CLI Interface

```bash
jsondiff --set-keys id file1.json file2.json
jsondiff --set-keys name,type file1.json file2.json  # Composite key
```

**Note:** `--set-keys` implies set mode (`-s`) for arrays containing objects with the specified key(s). Arrays without matching keys use the default array mode.

## jd Equivalence

```bash
# jd command:
jd -set -setkeys id file1.json file2.json

# jsondiff equivalent:
jsondiff --set-keys id file1.json file2.json
```

## Example

```json
// left.json
{"users": [{"id": 1, "name": "Alice"}, {"id": 2, "name": "Bob"}]}

// right.json
{"users": [{"id": 2, "name": "Robert"}, {"id": 1, "name": "Alice"}]}
```

**Without `--set-keys`:** Shows all fields changed (position mismatch)

**With `--set-keys id`:** Only shows `$.users[id=2].name` changed from "Bob" to "Robert"

## Path Notation

Key-matched elements use bracket notation with the key value:
- `$.users[id=1].name` instead of `$.users[0].name`
- `$.items[name=foo,type=bar]` for composite keys

