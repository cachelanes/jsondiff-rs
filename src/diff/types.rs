use sonic_rs::Value;
use std::collections::HashMap;
use std::fmt;

/// Represents a single difference between two JSON values
#[derive(Debug, Clone)]
pub enum DiffOp {
    /// Value was added (only in right)
    Added { path: JsonPath, value: Value },
    /// Value was removed (only in left)
    Removed { path: JsonPath, value: Value },
    /// Value was modified (different in left and right)
    Modified {
        path: JsonPath,
        old_value: Value,
        new_value: Value,
    },
}

/// JSON path representation (e.g., "$.users[0].name")
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct JsonPath {
    segments: Vec<PathSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathSegment {
    Root,
    Key(String),
    Index(usize),
    SetMarker,
    MultiSetMarker,
    /// Key-based match for set-key mode, e.g. [id=1] or [name=foo,type=bar]
    KeyMatch(Vec<(String, String)>),
}

impl JsonPath {
    pub fn root() -> Self {
        Self {
            segments: vec![PathSegment::Root],
        }
    }

    pub fn append_key(&self, key: &str) -> Self {
        let mut new_path = self.clone();
        new_path.segments.push(PathSegment::Key(key.to_string()));
        new_path
    }

    pub fn append_index(&self, index: usize) -> Self {
        let mut new_path = self.clone();
        new_path.segments.push(PathSegment::Index(index));
        new_path
    }

    pub fn append_set_marker(&self) -> Self {
        let mut new_path = self.clone();
        new_path.segments.push(PathSegment::SetMarker);
        new_path
    }

    pub fn append_multiset_marker(&self) -> Self {
        let mut new_path = self.clone();
        new_path.segments.push(PathSegment::MultiSetMarker);
        new_path
    }

    pub fn append_key_match(&self, keys: Vec<(String, String)>) -> Self {
        let mut new_path = self.clone();
        new_path.segments.push(PathSegment::KeyMatch(keys));
        new_path
    }

    /// Convert path to a dot-separated key-only string for set-key config lookup.
    /// Strips Root, Index, SetMarker, MultiSetMarker, KeyMatch segments.
    /// e.g. $.users[id=1].orders → "users.orders"
    pub fn to_set_key_path(&self) -> String {
        self.segments
            .iter()
            .filter_map(|s| match s {
                PathSegment::Key(k) => Some(k.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(".")
    }
}

impl fmt::Display for JsonPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for segment in &self.segments {
            match segment {
                PathSegment::Root => write!(f, "$")?,
                PathSegment::Key(k) => {
                    if k.contains('.') || k.contains('[') || k.contains(' ') || k.contains('"') {
                        write!(f, "[\"{}\"]", k)?;
                    } else {
                        write!(f, ".{}", k)?;
                    }
                }
                PathSegment::Index(i) => write!(f, "[{}]", i)?,
                PathSegment::SetMarker => write!(f, "[{{}}]")?,
                PathSegment::MultiSetMarker => write!(f, "[[{{}}]]")?,
                PathSegment::KeyMatch(pairs) => {
                    write!(f, "[")?;
                    for (i, (key, val)) in pairs.iter().enumerate() {
                        if i > 0 {
                            write!(f, ",")?;
                        }
                        // Quote values that contain commas, equals, or quotes
                        if val.contains(',') || val.contains('=') || val.contains('"') {
                            write!(f, "{}=\"{}\"", key, val)?;
                        } else {
                            write!(f, "{}={}", key, val)?;
                        }
                    }
                    write!(f, "]")?;
                }
            }
        }
        Ok(())
    }
}

/// Configuration for set-key based array matching
#[derive(Debug, Clone)]
pub struct SetKeyConfig {
    /// Map from array path (dot-separated) to key field names
    /// e.g. "users" -> ["id"], "orders" -> ["order_id", "type"]
    pub paths: HashMap<String, Vec<String>>,
    pub allow_missing: bool,
    pub allow_duplicates: bool,
}

/// Configuration for the diff algorithm
#[derive(Debug, Clone)]
pub struct DiffConfig {
    pub array_mode: ArrayCompareMode,
    pub ordered_objects: bool,
    pub set_keys: Option<SetKeyConfig>,
}

impl Default for DiffConfig {
    fn default() -> Self {
        Self {
            array_mode: ArrayCompareMode::Ordered,
            ordered_objects: false,
            set_keys: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum ArrayCompareMode {
    #[default]
    Ordered,
    Set,
    MultiSet,
}

/// Complete diff result
#[derive(Debug)]
pub struct DiffResult {
    pub operations: Vec<DiffOp>,
    pub stats: DiffStats,
}

#[derive(Debug, Default)]
pub struct DiffStats {
    pub added: usize,
    pub removed: usize,
    pub modified: usize,
}

