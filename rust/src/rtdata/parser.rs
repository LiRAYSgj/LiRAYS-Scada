use crate::rtdata::namespace::{AddCommand, Command, DelCommand, GetCommand, ListCommand, SetCommand, Value};
use winnow::{
    Parser, Result, ascii::{multispace0, multispace1}, combinator::{alt, delimited, repeat, separated}, error::{ContextError, InputError}, stream::AsChar, token::{any, take_till, take_while}
};

fn parse_quoted(input: &mut &str) -> Result<String> {
    delimited(
        '"',
        repeat(0.., alt(("\\\"".value('"'), any.verify(|&c| c != '"')))),
        '"'
    )
    .parse_next(input)
}

fn parse_unquoted<'s>(input: &mut &'s str) -> Result<String> {
    take_till(1.., (' ', '\t', '\n', '\r', '"'))
    .map(ToString::to_string)
    .parse_next(input)
}

pub fn parse_args(input: &mut &str) -> std::result::Result<Vec<String>, String> {
    multispace0.parse_next(input).map_err(|e: ContextError| {e.to_string()})?;
    let list: Vec<String> = separated(0..,
        alt((parse_quoted, parse_unquoted)),
        multispace1
    )
    .parse_next(input).map_err(|e| e.to_string())?;
    multispace0.parse_next(input).map_err(|e: ContextError| e.to_string())?;

    Ok(list)
}

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
