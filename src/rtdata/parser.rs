use winnow::{
    Parser,
    ascii::dec_uint,
    combinator::opt,
    error::ContextError,
};

/// Parse suffix pattern like `name[1:10:2]` returning numeric slice start/end/step,
/// or `name[a,b,c]` returning a list of string options. The parsed suffix is stripped
/// from `input` when successful.
pub fn parse_repeated_name(input: &mut &str) -> (Option<usize>, Option<usize>, Option<usize>, Option<Vec<String>>) {
    let mut start = None;
    let mut end = None;
    let mut step = None;
    let mut options: Option<Vec<String>> = None;

    if let Some(idx) = input.rfind('[') {
        let slice_part = &input[idx..];
        let mut slice_input = slice_part;

        let mut parser = (
            "[",
            opt(dec_uint::<_, usize, ContextError>),
            ":",
            opt(dec_uint::<_, usize, ContextError>),
            opt((
                ":",
                opt(dec_uint::<_, usize, ContextError>)
            )),
            "]"
        );

        if let Ok((_, s, _, e, step_part, _)) = parser.parse_next(&mut slice_input) {
            if slice_input.is_empty() {
                start = s;
                end = e;
                step = step_part.and_then(|(_, st)| st);
                *input = &input[..idx];
                return (start, end, step, options);
            }
        }

        // Not numeric slice: try comma-separated options
        if slice_part.ends_with(']') && slice_part.len() >= 2 && slice_part.contains(',') {
            let inner = &slice_part[1..slice_part.len() - 1];
            let parts: Vec<String> = inner
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !parts.is_empty() {
                options = Some(parts);
                *input = &input[..idx];
            }
        }
    }

    (start, end, step, options)
}

/// Expand `input` into a list of names, either by numeric range or explicit options.
pub fn clone_name(input: &str, start: Option<usize>, stop: Option<usize>, step: Option<usize>, options: Option<Vec<String>>) -> Vec<String> {
    if let Some(opts) = options {
        return opts.into_iter().map(|opt| format!("{input}{opt}")).collect();
    }

    if let Some(end) = stop {
        let start_val = start.unwrap_or(1);
        let step_val = step.unwrap_or(1);

        if step_val == 0 {
            return vec![input.to_string()];
        }

        let mut result = Vec::new();
        let mut current = start_val;
        while current <= end {
            result.push(format!("{input}{current}"));
            current += step_val;
        }
        result
    } else {
        vec![input.to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_repeated_name() {
        let mut input = "flat-string";
        assert_eq!(parse_repeated_name(&mut input), (None, None, None, None));
        assert_eq!(input, "flat-string");

        let mut input = "some-string[:10]";
        assert_eq!(parse_repeated_name(&mut input), (None, Some(10), None, None));
        assert_eq!(input, "some-string");

        let mut input = "some-string[24:10]";
        assert_eq!(parse_repeated_name(&mut input), (Some(24), Some(10), None, None));
        assert_eq!(input, "some-string");

        let mut input = "some-string[:10:2]";
        assert_eq!(parse_repeated_name(&mut input), (None, Some(10), Some(2), None));
        assert_eq!(input, "some-string");

        let mut input = "some-string[2:10:2]";
        assert_eq!(parse_repeated_name(&mut input), (Some(2), Some(10), Some(2), None));
        assert_eq!(input, "some-string");

        let mut input = "some-string[2:]";
        assert_eq!(parse_repeated_name(&mut input), (Some(2), None, None, None));
        assert_eq!(input, "some-string");

        let mut input = "some-string[2::3]";
        assert_eq!(parse_repeated_name(&mut input), (Some(2), None, Some(3), None));
        assert_eq!(input, "some-string");

        let mut input = "some-string[:]";
        assert_eq!(parse_repeated_name(&mut input), (None, None, None, None));
        assert_eq!(input, "some-string");

        let mut input = "some-string[::]";
        assert_eq!(parse_repeated_name(&mut input), (None, None, None, None));
        assert_eq!(input, "some-string");

        let mut input = "some-string[10]";
        assert_eq!(parse_repeated_name(&mut input), (None, None, None, None));
        assert_eq!(input, "some-string[10]");

        let mut input = "some-string[a:b]";
        assert_eq!(parse_repeated_name(&mut input), (None, None, None, None));
        assert_eq!(input, "some-string[a:b]");

        let mut input = "some-string[10:5]";
        assert_eq!(parse_repeated_name(&mut input), (Some(10), Some(5), None, None));
        assert_eq!(input, "some-string");

        let mut input = "some-string[10:15:10]";
        assert_eq!(parse_repeated_name(&mut input), (Some(10), Some(15), Some(10), None));
        assert_eq!(input, "some-string");

        let mut input = "some-string[2::2]";
        assert_eq!(parse_repeated_name(&mut input), (Some(2), None, Some(2), None));
        assert_eq!(input, "some-string");

        let mut input = "my_name[ab, xz, kk]";
        assert_eq!(parse_repeated_name(&mut input), (None, None, None, Some(vec!["ab".into(), "xz".into(), "kk".into()])));
        assert_eq!(input, "my_name");
    }

    #[test]
    fn test_clone_name() {
        let names = clone_name("base", Some(1), Some(3), Some(1), None);
        assert_eq!(names, vec!["base1", "base2", "base3"]);

        let names_opts = clone_name("base", None, None, None, Some(vec!["ab".into(), "xz".into(), "kk".into()]));
        assert_eq!(names_opts, vec!["baseab", "basexz", "basekk"]);
    }
}
