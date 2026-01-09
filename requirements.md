# JsonDiff - The prettiest, handiest JSON Differ

# Primary Requirements

In order, with most to least importance, we must:
- Ensure that something like this doesn't already exist
- Ensure that the output is useful
- Ensure that the output is beautiful
- Focus on how we handle ordering
    - Objects should be considered unordered by default: match regardless of ordering
    - Arrays should be considered ordered by default
    - Flags should be provided for bypassing both of the above
- Focus on speed: be lightning fast
- Leverage modern, powerful tools and libraries available in the rust ecosystem

# Secondary Requirements

- Support for specifying set keys
- Support for ignoring paths
- Support for NOT ignoring subpaths of ignored paths

