use std::collections::HashMap;

pub struct QueryInfo<'a> {
    pub prefix: &'a str,
    pub query_text: &'a str,
    pub statement_names: &'a[&'a str],
}

impl QueryInfo<'_> {
    pub const fn new(prefix: &'static str, query_text: &'static str, statement_names: &'static[&'static str]) -> Self {
        Self {
            prefix,
            query_text,
            statement_names,
        }
    }

    pub fn compose_query(queries: &HashMap<&'static str, QueryInfo<'static>>) -> String {
        let mut query = String::new();
        for (_, query_info) in queries {
            query.push_str(&query_info.query_text);
            query.push_str("\n");
        }
        query
    }
}
