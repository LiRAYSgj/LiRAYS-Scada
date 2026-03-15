use winnow::{
    Parser,
    ascii::dec_uint,
    combinator::opt,
    error::ContextError,
};

pub fn parse_repeated_name(input: &mut &str) -> (Option<usize>, Option<usize>, Option<usize>) {
    let mut start = None;
    let mut end = None;
    let mut step = None;

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
            }
        }
    }

    (start, end, step)
}

pub fn clone_name(input: &str, start: Option<usize>, stop: Option<usize>, step: Option<usize>) -> Vec<String> {
    if let Some(end) = stop {
        let start_val = start.unwrap_or(1);
        let step_val = step.unwrap_or(1);

        if step_val == 0 {
            return vec![input.to_string()];
        }

        let mut result = Vec::new();
        let mut current = start_val;
        while current <= end {
            result.push(format!("{input}_{current}"));
            current += step_val;
        }
        result
    } else {
        vec![input.to_string()]
    }
}

// use winnow::{
//     Parser,
//     Result,
//     ascii::{multispace0, multispace1, dec_uint},
//     combinator::{alt, opt, delimited, repeat, separated},
//     error::ContextError,
//     stream::AsChar,
//     token::{any, take_till, take_while, take_until}
// };
// fn parse_quoted(input: &mut &str) -> Result<String> {
//     delimited(
//         '"',
//         repeat(0.., alt(("\\\"".value('"'), any.verify(|&c| c != '"')))),
//         '"'
//     )
//     .parse_next(input)
// }

// fn parse_unquoted<'s>(input: &mut &'s str) -> Result<String> {
//     take_till(1.., (' ', '\t', '\n', '\r', '"'))
//     .map(ToString::to_string)
//     .parse_next(input)
// }

// pub fn parse_args(input: &mut &str) -> std::result::Result<Vec<String>, String> {
//     multispace0.parse_next(input).map_err(|e: ContextError| {e.to_string()})?;
//     let list: Vec<String> = separated(0..,
//         alt((parse_quoted, parse_unquoted)),
//         multispace1
//     )
//     .parse_next(input).map_err(|e| e.to_string())?;
//     multispace0.parse_next(input).map_err(|e: ContextError| e.to_string())?;

//     Ok(list)
// }

// pub fn parse_command<'s>(input: &mut &'s str) -> Result<Command, String> {
//     multispace0.parse_next(input).map_err(|e: ContextError| {e.to_string()})?;
//     let cmd_id = take_while(0.., AsChar::is_alphanum)
//         .parse_next(input)
//         .map_err(|e: winnow::error::ErrMode<InputError<&str>>| e.to_string())?
//         .to_string();
//     multispace0.parse_next(input).map_err(|e: ContextError| {e.to_string()})?;
//     let cmd = take_while(0.., AsChar::is_alphanum)
//         .parse_next(input)
//         .map_err(|e: winnow::error::ErrMode<InputError<&str>>| e.to_string())?
//         .to_uppercase();
//     match cmd.as_str() {
//         "ADD" => {
//             let args = parse_args(input)?;
//             let mut it = args.into_iter();

//             let parent_id = it.next().ok_or("Missing parent id")?;
//             let var_names: Vec<String> = it.collect();
//             if var_names.len() > 0 {
//                 Ok(Command::ADD(AddCommand { cmd_id, parent_id, var_names }))
//             } else {
//                 Err(format!("Missing var names"))
//             }
//         }
//         "LS" => {
//             let args = parse_args(input)?;
//             let mut it = args.into_iter();

//             let var_id = it.next();
//             Ok(Command::LIST(ListCommand { cmd_id, var_id }))
//         }
//         "SET" => {
//             let args = parse_args(input)?;

//             let mut var_ids_values = vec![];
//             for batch in args.chunks_exact(3) {
//                 let var_id = &batch[0];
//                 let var_type = &batch[1];
//                 let val = &batch[2];
//                 match var_type.to_lowercase().as_str() {
//                     "i" => {
//                         match val.parse::<i128>() {
//                             Ok(v) => var_ids_values.push((var_id.to_owned(), Value::Integer(v))),
//                             Err(e) => return Err(format!("Invalid int value: {e}"))
//                         }
//                     }
//                     "f" => {
//                         match val.parse::<f64>() {
//                             Ok(v) => var_ids_values.push((var_id.to_owned(), Value::Float(v))),
//                             Err(e) => return Err(format!("Invalid float value: {e}"))
//                         }
//                     }
//                     "t" => {
//                         var_ids_values.push((var_id.to_owned(), Value::Text(val.to_owned())));
//                     }
//                     "b" => {
//                         if val == "false" {
//                             var_ids_values.push((var_id.to_owned(), Value::Boolean(false)));
//                         } else if val == "true" {
//                             var_ids_values.push((var_id.to_owned(), Value::Boolean(true)));
//                         } else {
//                             return Err(format!("Invalid bool value: {val}. Must be true/false"));
//                         }
//                     }
//                     _ => return Err(format!("Invalid type specifier: {var_type}. Must be i/f/t/b"))
//                 }
//             }
//             Ok(Command::SET(SetCommand { cmd_id, var_ids_values }))
//         }
//         "GET" => {
//             let var_ids = parse_args(input)?;
//             Ok(Command::GET(GetCommand { cmd_id, var_ids }))
//         }
//         "DEL" => {
//             let var_ids = parse_args(input)?;
//             Ok(Command::DEL(DelCommand { cmd_id, var_ids }))
//         }
//         _ => Err(format!("Invalid command {cmd}"))
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_repeated_name() {
        // "flat-string" -> (None, None, None)
        let mut input = "flat-string";
        assert_eq!(parse_repeated_name(&mut input), (None, None, None));
        assert_eq!(input, "flat-string");

        // "some-string[:END]" -> (None, Some(END), None)
        let mut input = "some-string[:10]";
        assert_eq!(parse_repeated_name(&mut input), (None, Some(10), None));
        assert_eq!(input, "some-string");

        // "some-string[START:END]" -> (Some(START), Some(END), None)
        let mut input = "some-string[24:10]";
        assert_eq!(parse_repeated_name(&mut input), (Some(24), Some(10), None));
        assert_eq!(input, "some-string");

        // "some-string[:END:STEP]" -> (None, Some(END), Some(STEP))
        let mut input = "some-string[:10:2]";
        assert_eq!(parse_repeated_name(&mut input), (None, Some(10), Some(2)));
        assert_eq!(input, "some-string");

        // "some-string[START:END:STEP]" -> (Some(START), Some(END), Some(STEP))
        let mut input = "some-string[2:10:2]";
        assert_eq!(parse_repeated_name(&mut input), (Some(2), Some(10), Some(2)));
        assert_eq!(input, "some-string");

        // Additional edge cases
        let mut input = "some-string[2:]";
        assert_eq!(parse_repeated_name(&mut input), (Some(2), None, None));
        assert_eq!(input, "some-string");

        let mut input = "some-string[2::3]";
        assert_eq!(parse_repeated_name(&mut input), (Some(2), None, Some(3)));
        assert_eq!(input, "some-string");

        let mut input = "some-string[:]";
        assert_eq!(parse_repeated_name(&mut input), (None, None, None));
        assert_eq!(input, "some-string");

        let mut input = "some-string[::]";
        assert_eq!(parse_repeated_name(&mut input), (None, None, None));
        assert_eq!(input, "some-string");

        // Non-matching forms should not modify input and return (None, None, None)
        let mut input = "some-string[10]";
        assert_eq!(parse_repeated_name(&mut input), (None, None, None));
        assert_eq!(input, "some-string[10]");

        let mut input = "some-string[a:b]";
        assert_eq!(parse_repeated_name(&mut input), (None, None, None));
        assert_eq!(input, "some-string[a:b]");

        // start > stop
        let mut input = "some-string[10:5]";
        assert_eq!(parse_repeated_name(&mut input), (Some(10), Some(5), None));
        assert_eq!(input, "some-string");

        // step > (stop - start)
        let mut input = "some-string[10:15:10]";
        assert_eq!(parse_repeated_name(&mut input), (Some(10), Some(15), Some(10)));
        assert_eq!(input, "some-string");

        // missing stop but step is defined
        let mut input = "some-string[2::2]";
        assert_eq!(parse_repeated_name(&mut input), (Some(2), None, Some(2)));
        assert_eq!(input, "some-string");
    }

    #[test]
    fn test_clone_name() {
        // Without stop defined, it should just return the input
        assert_eq!(
            clone_name("var", None, None, None),
            vec!["var".to_string()]
        );

        // With stop only, assumes start=1 and step=1
        assert_eq!(
            clone_name("var", None, Some(3), None),
            vec!["var_1".to_string(), "var_2".to_string(), "var_3".to_string()]
        );

        // With start=0
        assert_eq!(
            clone_name("var", Some(0), Some(2), None),
            vec!["var_0".to_string(), "var_1".to_string(), "var_2".to_string()]
        );

        // With explicit step
        assert_eq!(
            clone_name("var", Some(2), Some(8), Some(3)),
            vec!["var_2".to_string(), "var_5".to_string(), "var_8".to_string()]
        );

        // Where start > stop
        assert_eq!(
            clone_name("var", Some(5), Some(2), None),
            Vec::<String>::new()
        );

        // Where step > (stop - start)
        assert_eq!(
            clone_name("var", Some(2), Some(5), Some(10)),
            vec!["var_2".to_string()]
        );

        // Missing stop but start and step provided (fallback to just input)
        assert_eq!(
            clone_name("var", Some(2), None, Some(10)),
            vec!["var".to_string()]
        );

        // start > stop and step > 0
        assert_eq!(
            clone_name("var", Some(10), Some(5), Some(1)),
            Vec::<String>::new()
        );

        // step is 0
        assert_eq!(
            clone_name("var", Some(1), Some(5), Some(0)),
            vec!["var".to_string()]
        );
    }
}
