use super::utils::normalize_path;

/// Canonical namespace path pattern. `*` matches any characters within one segment,
/// and a whole `**` segment matches zero or more segments.
#[derive(Clone, Debug)]
pub struct PathPattern {
    segments: Vec<String>,
    literal_prefix: String,
}

impl PathPattern {
    pub fn parse(pattern: &str) -> Result<Self, String> {
        if !pattern.starts_with('/') || pattern.contains('\0') || normalize_path(pattern) != pattern
        {
            return Err(format!("Invalid path pattern: {pattern:?}"));
        }
        let segments: Vec<String> = pattern.split('/').skip(1).map(String::from).collect();
        if segments.iter().any(|s| s.contains("**") && s != "**") {
            return Err(format!("'**' must be a whole path segment: {pattern}"));
        }
        let literal_prefix = segments
            .iter()
            .take_while(|s| !s.contains('*'))
            .map(|s| format!("/{s}"))
            .collect();
        Ok(Self {
            segments,
            literal_prefix,
        })
    }

    pub fn is_exact(&self) -> bool {
        self.segments.iter().all(|s| !s.contains('*'))
    }

    /// Path before the first wildcard segment: every match lies at or below it.
    /// Empty when the first segment is a wildcard; the whole path for exact patterns.
    pub fn literal_prefix(&self) -> &str {
        &self.literal_prefix
    }

    pub fn matches(&self, path: &str) -> bool {
        let parts: Vec<&str> = path.split('/').skip(1).collect();
        match_segments(&self.segments, &parts)
    }

    /// Whether a change inside `folder` can add or remove matches of this pattern.
    pub fn affected_by(&self, folder: &str) -> bool {
        let folder = normalize_path(folder);
        is_within(&self.literal_prefix, &folder) || is_within(&folder, &self.literal_prefix)
    }
}

fn is_within(path: &str, ancestor: &str) -> bool {
    ancestor.is_empty()
        || path
            .strip_prefix(ancestor)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

// Tracks which path prefixes the pattern consumed so far can match, so `**` segments
// cost O(pattern × path) instead of backtracking exponentially.
fn match_segments(pattern: &[String], path: &[&str]) -> bool {
    let mut reachable = vec![false; path.len() + 1];
    reachable[0] = true;
    for segment in pattern {
        if segment == "**" {
            let mut seen = false;
            for slot in reachable.iter_mut() {
                seen |= *slot;
                *slot = seen;
            }
        } else {
            let mut next = vec![false; path.len() + 1];
            for (index, part) in path.iter().enumerate() {
                next[index + 1] = reachable[index] && match_glob(segment, part);
            }
            reachable = next;
        }
    }
    reachable[path.len()]
}

fn match_glob(pattern: &str, text: &str) -> bool {
    let mut parts = pattern.split('*');
    let Some(mut rest) = text.strip_prefix(parts.next().unwrap_or_default()) else {
        return false;
    };
    let parts: Vec<&str> = parts.collect();
    let Some((last, middle)) = parts.split_last() else {
        return rest.is_empty();
    };
    for part in middle {
        let Some(index) = rest.find(part) else {
            return false;
        };
        rest = &rest[index + part.len()..];
    }
    rest.ends_with(last)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matches(pattern: &str, path: &str) -> bool {
        PathPattern::parse(pattern).unwrap().matches(path)
    }

    #[test]
    fn wildcards_match_by_segment() {
        assert!(matches("/plant/temp", "/plant/temp"));
        assert!(!matches("/plant/temp", "/plant/temp2"));
        assert!(matches("/*", "/temp"));
        assert!(!matches("/*", "/plant/temp"));
        assert!(matches("/plant/*/temp", "/plant/line1/temp"));
        assert!(!matches("/plant/*/temp", "/plant/a/b/temp"));
        assert!(matches("/line*/temp", "/line12/temp"));
        assert!(matches("/*a*b*", "/xaxxb"));
        assert!(!matches("/a*a", "/a"));
        assert!(matches("/**", "/a/b/c"));
        assert!(matches("/plant/**", "/plant/a"));
        assert!(!matches("/plant/**", "/plantx/a"));
        assert!(matches("/plant/**/temp", "/plant/temp"));
        assert!(matches("/plant/**/temp", "/plant/a/b/temp"));
        assert!(!matches("/plant/**/temp", "/plant/a/b/pressure"));
    }

    #[test]
    fn repeated_double_wildcards_do_not_backtrack_exponentially() {
        let pattern = format!("{}/z", "/**".repeat(40));
        let path = format!("{}/y", "/a".repeat(200));
        assert!(!matches(&pattern, &path));
        assert!(matches(&pattern, &format!("{}/z", "/a".repeat(200))));
    }

    #[test]
    fn invalid_patterns_are_rejected() {
        for pattern in ["", "/", "plant/*", "/plant//*", "/plant/", "/a**", "/a\0b"] {
            assert!(PathPattern::parse(pattern).is_err(), "{pattern:?}");
        }
    }

    #[test]
    fn literal_prefix_limits_relevant_tree_changes() {
        let pattern = PathPattern::parse("/plant/line*/temp").unwrap();
        assert_eq!(pattern.literal_prefix(), "/plant");
        assert!(pattern.affected_by("/plant/line1"));
        assert!(pattern.affected_by("/"));
        assert!(!pattern.affected_by("/plantx"));
        assert!(!pattern.affected_by("/other/plant"));
        let exact = PathPattern::parse("/plant/temp").unwrap();
        assert!(exact.is_exact());
        assert!(exact.affected_by("/plant"));
        assert!(!exact.affected_by("/plant/line1"));
    }
}
