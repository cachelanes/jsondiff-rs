# Selectively Exclude Paths

Exclude specific paths from comparison, with ability to re-include subpaths.

## CLI Interface

```bash
# Ignore paths
jsondiff --ignore "metadata.timestamp" file1.json file2.json
jsondiff --ignore "$.results[*].latency_ms" file1.json file2.json
jsondiff -I "id" -I "created_at" file1.json file2.json  # Multiple

# Include subpaths (overrides ignore)
jsondiff --ignore "metadata" --include "metadata.version" file1.json file2.json
```

## jd Equivalence

```bash
# jd command (ignore a path):
jd -opts='[{"@": ["results", "tcp_latency_ms"], "^":["DIFF_OFF"]}]' a.json b.json

# jsondiff equivalent:
jsondiff --ignore results.tcp_latency_ms a.json b.json
```

## Path Pattern Syntax

| Pattern      | Matches                                              |
|--------------|------------------------------------------------------|
| `foo.bar`    | Exact path `$.foo.bar`                               |
| `foo.*.bar`  | Single wildcard: `$.foo.x.bar`, `$.foo.y.bar`        |
| `foo[*].bar` | Array index wildcard: `$.foo[0].bar`, `$.foo[1].bar` |
| `**.bar`     | Recursive wildcard: `$.bar`, `$.x.bar`, `$.x.y.bar`  |

## Precedence

`--include` takes precedence over `--ignore`:

```bash
jsondiff --ignore "results" --include "results.summary" file1.json file2.json
# Ignores all of results.* except results.summary
```

