use crate::at_commands::execute::parse_words_from_line;


pub fn query_line_args(line: &String, cursor_rel: i64, cursor_line_start: i64, at_command_names: &Vec<String>) -> Vec<QueryLineArg> {
    let mut args: Vec<QueryLineArg> = vec![];
    for (text, pos1, pos2) in parse_words_from_line(line).iter().rev().cloned() {
        if at_command_names.contains(&text) && args.iter().any(|x|(x.value.contains("@") && x.focused) || at_command_names.contains(&x.value)) {
            break;
        }
        let mut x = QueryLineArg {
            value: text.clone(),
            pos1: pos1 as i64, pos2: pos2 as i64,
            focused: false,
        };
        x.focused = cursor_rel >= x.pos1 && cursor_rel <= x.pos2;
        x.pos1 += cursor_line_start;
        x.pos2 += cursor_line_start;
        args.push(x)
    }
    args.iter().rev().cloned().collect::<Vec<_>>()
}

#[derive(Debug, Clone)]
pub struct QueryLineArg {
    pub value: String,
    pub pos1: i64,
    pub pos2: i64,
    pub focused: bool,
}
