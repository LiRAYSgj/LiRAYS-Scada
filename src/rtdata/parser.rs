use winnow::{Parser, ascii::dec_uint, combinator::opt, error::ContextError};

/// Parse patterns like:
/// - `name[1:10:2]` (numeric range)
/// - `name[a,b,c]` (explicit options)
/// and now supports a trailing suffix after the bracket:
/// - `name[1:3]tail` -> base `nametail` with range 1..3
/// - `name[a,b]tail` -> base `nametail` with options.
///
/// Returns (prefix, suffix, start, end, step, options).
pub fn parse_repeated_name(
    input: &str,
) -> (
    String,
    String,
    Option<usize>,
    Option<usize>,
    Option<usize>,
    Option<Vec<String>>,
) {
    let mut prefix_out = input.to_string();
    let mut suffix_out = String::new();
    let mut start = None;
    let mut end = None;
    let mut step = None;
    let mut options: Option<Vec<String>> = None;

    if let (Some(open_idx), Some(close_idx)) = (input.find('['), input.rfind(']')) {
        if close_idx > open_idx {
            let prefix = &input[..open_idx];
            let bracket = &input[open_idx..=close_idx];
            let suffix = &input[close_idx + 1..];

            let mut slice_input = bracket;
            let mut parser = (
                "[",
                opt(dec_uint::<_, usize, ContextError>),
                ":",
                opt(dec_uint::<_, usize, ContextError>),
                opt((":", opt(dec_uint::<_, usize, ContextError>))),
                "]",
            );

            if let Ok((_, s, _, e, step_part, _)) = parser.parse_next(&mut slice_input) {
                if slice_input.is_empty() {
                    start = s;
                    end = e;
                    step = step_part.and_then(|(_, st)| st);
                    prefix_out = prefix.to_string();
                    suffix_out = suffix.to_string();
                    return (prefix_out, suffix_out, start, end, step, options);
                }
            }

            if bracket.ends_with(']') && bracket.len() >= 2 && bracket.contains(',') {
                let inner = &bracket[1..bracket.len() - 1];
                let parts: Vec<String> = inner
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if !parts.is_empty() {
                    options = Some(parts);
                    prefix_out = prefix.to_string();
                    suffix_out = suffix.to_string();
                }
            }
        }
    }

    (prefix_out, suffix_out, start, end, step, options)
}

/// Expand into a list of names, either by numeric range or explicit options.
/// Names are built as `prefix + value + suffix`.
pub fn clone_name(
    prefix: &str,
    suffix: &str,
    start: Option<usize>,
    stop: Option<usize>,
    step: Option<usize>,
    options: Option<Vec<String>>,
) -> Vec<String> {
    if let Some(opts) = options {
        return opts
            .into_iter()
            .map(|opt| format!("{prefix}{opt}{suffix}"))
            .collect();
    }

    if let Some(end) = stop {
        let start_val = start.unwrap_or(1);
        let step_val = step.unwrap_or(1);

        if step_val == 0 {
            return vec![format!("{prefix}{suffix}")];
        }

        let mut result = Vec::new();
        let mut current = start_val;
        while current <= end {
            result.push(format!("{prefix}{current}{suffix}"));
            current += step_val;
        }
        result
    } else {
        vec![format!("{prefix}{suffix}")]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_repeated_name() {
        let (pfx, sfx, s, e, stp, opts) = parse_repeated_name("flat-string");
        assert_eq!(
            (pfx, sfx, s, e, stp, opts),
            ("flat-string".into(), "".into(), None, None, None, None)
        );

        let (pfx, sfx, s, e, stp, opts) = parse_repeated_name("some-string[:10]");
        assert_eq!(
            (pfx + &sfx, s, e, stp, opts),
            ("some-string".into(), None, Some(10), None, None)
        );

        let (pfx, sfx, s, e, stp, opts) = parse_repeated_name("some-string[24:10]");
        assert_eq!(
            (pfx + &sfx, s, e, stp, opts),
            ("some-string".into(), Some(24), Some(10), None, None)
        );

        let (pfx, sfx, s, e, stp, opts) = parse_repeated_name("some-string[:10:2]");
        assert_eq!(
            (pfx + &sfx, s, e, stp, opts),
            ("some-string".into(), None, Some(10), Some(2), None)
        );

        let (pfx, sfx, s, e, stp, opts) = parse_repeated_name("some-string[2:10:2]");
        assert_eq!(
            (pfx + &sfx, s, e, stp, opts),
            ("some-string".into(), Some(2), Some(10), Some(2), None)
        );

        let (pfx, sfx, s, e, stp, opts) = parse_repeated_name("some-string[2:]");
        assert_eq!(
            (pfx + &sfx, s, e, stp, opts),
            ("some-string".into(), Some(2), None, None, None)
        );

        let (pfx, sfx, s, e, stp, opts) = parse_repeated_name("some-string[2::3]");
        assert_eq!(
            (pfx + &sfx, s, e, stp, opts),
            ("some-string".into(), Some(2), None, Some(3), None)
        );

        let (pfx, sfx, s, e, stp, opts) = parse_repeated_name("some-string[:]");
        assert_eq!(
            (pfx + &sfx, s, e, stp, opts),
            ("some-string".into(), None, None, None, None)
        );

        let (pfx, sfx, s, e, stp, opts) = parse_repeated_name("some-string[::]");
        assert_eq!(
            (pfx + &sfx, s, e, stp, opts),
            ("some-string".into(), None, None, None, None)
        );

        let (pfx, sfx, s, e, stp, opts) = parse_repeated_name("some-string[10]");
        assert_eq!(
            (pfx + &sfx, s, e, stp, opts),
            ("some-string[10]".into(), None, None, None, None)
        );

        let (pfx, sfx, s, e, stp, opts) = parse_repeated_name("some-string[a:b]");
        assert_eq!(
            (pfx + &sfx, s, e, stp, opts),
            ("some-string[a:b]".into(), None, None, None, None)
        );

        let (pfx, sfx, s, e, stp, opts) = parse_repeated_name("some-string[10:5]");
        assert_eq!(
            (pfx + &sfx, s, e, stp, opts),
            ("some-string".into(), Some(10), Some(5), None, None)
        );

        let (pfx, sfx, s, e, stp, opts) = parse_repeated_name("some-string[10:15:10]");
        assert_eq!(
            (pfx + &sfx, s, e, stp, opts),
            ("some-string".into(), Some(10), Some(15), Some(10), None)
        );

        let (pfx, sfx, s, e, stp, opts) = parse_repeated_name("some-string[2::2]");
        assert_eq!(
            (pfx + &sfx, s, e, stp, opts),
            ("some-string".into(), Some(2), None, Some(2), None)
        );

        let (pfx, sfx, s, e, stp, opts) = parse_repeated_name("my_name[ab, xz, kk]");
        assert_eq!(
            (pfx + &sfx, s, e, stp, opts),
            (
                "my_name".into(),
                None,
                None,
                None,
                Some(vec!["ab".into(), "xz".into(), "kk".into()])
            )
        );

        let (pfx, sfx, s, e, stp, opts) = parse_repeated_name("name[1:3]tail");
        assert_eq!(
            (pfx, sfx, s, e, stp, opts),
            ("name".into(), "tail".into(), Some(1), Some(3), None, None)
        );

        let (pfx, sfx, s, e, stp, opts) = parse_repeated_name("name[a,b]tail");
        assert_eq!(
            (pfx, sfx, s, e, stp, opts),
            (
                "name".into(),
                "tail".into(),
                None,
                None,
                None,
                Some(vec!["a".into(), "b".into()])
            )
        );
    }

    #[test]
    fn test_clone_name() {
        let names = clone_name("base", "", Some(1), Some(3), Some(1), None);
        assert_eq!(names, vec!["base1", "base2", "base3"]);

        let names_opts = clone_name(
            "base",
            "",
            None,
            None,
            None,
            Some(vec!["ab".into(), "xz".into(), "kk".into()]),
        );
        assert_eq!(names_opts, vec!["baseab", "basexz", "basekk"]);

        // With suffix preserved separately
        let (pfx, sfx, s, e, stp, opts) = parse_repeated_name("prefix[1:4]suffix");
        let names = clone_name(&pfx, &sfx, s, e, stp, opts);
        assert_eq!(
            names,
            vec![
                "prefix1suffix",
                "prefix2suffix",
                "prefix3suffix",
                "prefix4suffix"
            ]
        );

        let (pfx, sfx, s, e, stp, opts) = parse_repeated_name("prefix[a, b]suffix");
        let names = clone_name(&pfx, &sfx, s, e, stp, opts);
        assert_eq!(names, vec!["prefixasuffix", "prefixbsuffix"]);
    }
}
