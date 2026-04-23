use lirays_scada_proto::namespace::v1::{VarDataType, value::Typed};

pub struct CachedValue {
    pub val: Option<Typed>,
    pub dtype: VarDataType,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub options: Vec<String>,
    pub max_len: Option<u64>,
}

pub fn normalize_path(path: &str) -> String {
    let mut base = String::with_capacity(path.len());

    for part in path.split('/').filter(|s| !s.is_empty()) {
        base.push('/');
        base.push_str(part);
    }
    base
}

pub fn get_ancestors(path: &str) -> Vec<(String, String)> {
    let mut ancestors = vec![];
    let mut parent = String::from("/");

    for part in path.split('/').filter(|s| !s.is_empty()) {
        ancestors.push((parent.clone(), part.to_string()));
        parent.push_str(part);
        parent.push('/');
    }
    ancestors
}

pub fn get_parent_and_name(path: &str) -> (String, String) {
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    match parts.as_slice() {
        [] => (String::from("/"), String::new()),
        [name] => (String::from("/"), name.to_string()),
        [parent_parts @ .., name] => {
            let mut parent = String::with_capacity(path.len());
            for part in parent_parts {
                parent.push('/');
                parent.push_str(part);
            }
            (parent, name.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("/a/b/c/"), "/a/b/c");
        assert_eq!(normalize_path("//a////b/c/d"), "/a/b/c/d");
        assert_eq!(normalize_path("/a/b/c//d//"), "/a/b/c/d");
        assert_eq!(normalize_path(""), "");
        assert_eq!(normalize_path("/"), "");
        assert_eq!(normalize_path("//"), "");
        assert_eq!(normalize_path("///"), "");
    }

    #[test]
    fn test_get_ancestors() {
        let ancestors = get_ancestors("/a/b/c");
        assert_eq!(ancestors.len(), 3);
        assert_eq!(ancestors[0], (String::from("/"), String::from("a")));
        assert_eq!(ancestors[1], (String::from("/a/"), String::from("b")));
        assert_eq!(ancestors[2], (String::from("/a/b/"), String::from("c")));
    }

    #[test]
    fn test_get_parent_and_name() {
        // Test root path
        assert_eq!(
            get_parent_and_name("/"),
            (String::from("/"), String::from(""))
        );

        // Test single component
        assert_eq!(
            get_parent_and_name("/a"),
            (String::from("/"), String::from("a"))
        );

        // Test multiple components
        assert_eq!(
            get_parent_and_name("/a/b/c"),
            (String::from("/a/b"), String::from("c"))
        );

        // Test path with trailing slash
        assert_eq!(
            get_parent_and_name("/a/b/c/"),
            (String::from("/a/b"), String::from("c"))
        );
    }
}
