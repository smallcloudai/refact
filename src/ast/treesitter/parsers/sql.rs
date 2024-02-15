use crate::ast::treesitter::parsers::{AstConfig, Language};

pub struct SqlConfig;

impl Language for SqlConfig {
    fn make_ast_config() -> AstConfig {
        AstConfig {
            type_declaration_search_info: vec![],
            namespace_search_info: None,
            keywords: vec![
                "ABORT", "DECIMAL", "INTERVAL", "PRESERVE", "ALL", "DECODE", "INTO", "PRIMARY", "ALLOCATE", "DEFAULT",
                "LEADING", "RESET", "ANALYSE", "DESC", "LEFT", "REUSE", "ANALYZE", "DISTINCT", "LIKE", "RIGHT",
                "AND", "DISTRIBUTE", "LIMIT", "ROWS", "ANY", "DO", "LOAD", "SELECT", "AS", "ELSE", "LOCAL", "SESSION_USER",
                "ASC", "END", "LOCK", "SETOF", "SHOW", "BETWEEN", "EXCEPT", "MINUS", "SHOW", "BINARY", "EXCLUDE", "MOVE",
                "SOME", "BIT", "EXISTS", "NATURAL", "TABLE", "BOTH", "EXPLAIN", "NCHAR", "THEN", "CASE", "EXPRESS", "NEW",
                "TIES", "CAST", "EXTEND", "NOT", "TIME", "CHAR", "EXTERNAL", "NOTNULL", "TIMESTAMP", "CHARACTER",
                "EXTRACT", "NULL", "TO", "CHECK", "FALSE", "NULLS", "TRAILING", "CLUSTER", "FIRST", "NUMERIC",
                "TRANSACTION", "COALESCE", "FLOAT", "NVL", "TRIGGER", "COLLATE", "FOLLOWING", "NVL2", "TRIM",
                "COLLATION", "FOR", "OFF", "TRUE", "COLUMN", "FOREIGN", "OFFSET", "UNBOUNDED", "CONSTRAINT", "FROM",
                "OLD", "UNION", "COPY", "FULL", "ON", "UNIQUE", "CROSS", "FUNCTION", "ONLINE", "USER", "CURRENT",
                "GENSTATS", "ONLY", "USING", "CURRENT_CATALOG", "GLOBAL", "OR", "VACUM", "CURRENT_DATE", "GROUP",
                "ORDER", "VARCHAR", "CURRENT_DB", "HAVING", "OTHERS", "VERBOSE", "CURRENT_SCHEMA", "IDENTIFIER_CASE",
                "OUT", "VERSION", "CURRENT_SID", "ILIKE", "INNER", "OUTER", "VIEW", "CURRENT_TIME", "IN", "OVER", "WHEN",
                "CURRENT_TIMESTAMP", "INDEX", "OVERLAPS", "WHERE", "CURRENT_USER", "INITIALLY", "PARTITION", "WITH",
                "CURRENT_USERID", "INNER", "POSITION", "WRITE", "CURRENT_USEROID", "INOUT", "PRECEDING", "RESET",
                "DEALLOCATE", "INTERSECT", "PRECISION", "REUSE", "DEC"
            ].iter().map(|s| s.to_string()).collect(),
            keywords_types: vec![].iter().map(|s: &&str| s.to_string()).collect(),
        }
    }
}