#[derive(Clone)]
pub struct QueryLine {
    pub value: String,
    pub cursor_line_start: i64,
    pub args: Vec<QueryLineArg>,  // the first argument is the command, the rest are arguments
}

impl QueryLine {
    pub fn new(
        value: String,
        cursor_rel: i64,
        cursor_line_start: i64
    ) -> Self {
        QueryLine {
            value: value.clone(),
            cursor_line_start,
            args: parse_args_from_line(&value).iter_mut().map(|x| {
                x.pos2 += 1;
                x.focused = cursor_rel >= x.pos1 && cursor_rel <= x.pos2;
                x.pos1 += cursor_line_start;
                x.pos2 += cursor_line_start;
                x.clone()
            }).collect(),
        }
    }

    pub fn command(&self) -> Option<&QueryLineArg> {
        self.args.first()
    }

    pub fn get_args(&self) -> Vec<&QueryLineArg> {
        self.args.iter().skip(1).collect()
    }
}

#[derive(Clone)]
pub struct QueryLineArg {
    pub value: String,
    pub pos1: i64,
    pub pos2: i64,
    pub focused: bool,
    pub type_name: String,
}

fn parse_args_from_line(line: &String) -> Vec<QueryLineArg> {
    let mut pos1: i64 = -1;
    let mut value: String = "".to_string();
    let mut args: Vec<QueryLineArg> = vec![];
    for (idx, ch) in line.chars().enumerate() {
        let idx = idx as i64;
        if value.is_empty() && ch.to_string() != " " {
            pos1 = idx;
        }

        if ch.to_string() != " " {
            value.push(ch);
        }

        if pos1 != -1 && (ch.to_string() == " " || idx == (line.len() -1) as i64) {
            args.push(QueryLineArg{
                value: value.clone(),
                pos1,
                pos2: idx,
                focused: false,
                type_name: {
                    if value.starts_with("@") {
                        "command".to_string()
                    } else {
                        "arg".to_string()
                    }
                }
            });
            pos1 = -1;
            value = "".to_string();
        }
    }
    args
}
