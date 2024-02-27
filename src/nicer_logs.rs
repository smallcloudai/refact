pub fn first_n_chars(msg: &String, n: usize) -> String {
    let mut last_n_chars: String = msg.chars().take(n).collect();
    if last_n_chars.len() == n {
        last_n_chars.push_str("...");
    }
    return last_n_chars.replace("\n", "\\n");
}

pub fn last_n_chars(msg: &String, n: usize) -> String {
    let mut last_n_chars: String = msg.chars().rev().take(n).collect::<String>().chars().rev().collect();
    if last_n_chars.len() == n {
        last_n_chars.insert_str(0, "...");
    }
    return last_n_chars.replace("\n", "\\n");
}
