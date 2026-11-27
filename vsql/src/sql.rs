// Changes to the text of a statement before it's given to libvsql.

use std::borrow::Cow;

/// The statements of SQL that's separated by `;`, which libvsql executes one
/// at a time. A `;` in a string, a quoted identifier or a comment doesn't end
/// a statement, and a statement that's empty (or only comments) is left out.
pub fn split_statements(sql: &str) -> Vec<&str> {
    let bytes = sql.as_bytes();
    let mut statements = Vec::new();
    let mut start = 0;
    // Whether the statement has something other than spaces and comments.
    let mut empty = true;
    let mut i = 0;
    while i < bytes.len() {
        if let Some(end) = skip_quoted(sql, i) {
            // A comment that isn't closed is left for libvsql to report.
            empty &= bytes[i] == b'/' && sql[i + 2..end].ends_with("*/");
            i = end;
        } else if bytes[i] == b'-' && bytes.get(i + 1) == Some(&b'-') {
            i = line_end(sql, i);
        } else {
            if bytes[i] == b';' {
                if !empty {
                    statements.push(&sql[start..i]);
                }
                start = i + 1;
                empty = true;
            } else {
                empty &= bytes[i].is_ascii_whitespace();
            }
            i += 1;
        }
    }
    if !empty {
        statements.push(&sql[start..]);
    }

    statements
}

/// The statement without its `--` comments, which libvsql doesn't allow. A
/// comment is removed up to the end of its line (the line break is kept, so
/// the line numbers of errors are the same). A `--` in a string, a quoted
/// identifier or a `/* */` comment isn't a comment.
pub fn remove_dash_comments(sql: &str) -> Cow<'_, str> {
    if !sql.contains("--") {
        return Cow::Borrowed(sql);
    }

    let bytes = sql.as_bytes();
    let mut out = String::with_capacity(sql.len());
    // The start of the text that hasn't been copied to out yet.
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        if let Some(end) = skip_quoted(sql, i) {
            i = end;
        } else if bytes[i] == b'-' && bytes.get(i + 1) == Some(&b'-') {
            out.push_str(&sql[start..i]);
            i = line_end(sql, i);
            start = i;
        } else {
            i += 1;
        }
    }
    out.push_str(&sql[start.min(sql.len())..]);

    Cow::Owned(out)
}

/// The end of the string, quoted identifier or `/* */` comment that starts at
/// byte `i`, or `None` if there isn't one. One that isn't closed ends at the
/// end of the SQL (it's left for libvsql to report).
pub(crate) fn skip_quoted(sql: &str, i: usize) -> Option<usize> {
    let bytes = sql.as_bytes();
    let end = match bytes[i] {
        // A quote is escaped by doubling it, which is the same as two strings
        // in a row.
        quote @ (b'\'' | b'"') => match bytes[i + 1..].iter().position(|&b| b == quote) {
            Some(n) => i + 1 + n + 1,
            None => bytes.len(),
        },
        b'/' if bytes.get(i + 1) == Some(&b'*') => match sql[i + 2..].find("*/") {
            Some(n) => i + 2 + n + 2,
            None => bytes.len(),
        },
        _ => return None,
    };

    Some(end)
}

/// The end of the line (before its line break) that byte `i` is on.
pub(crate) fn line_end(sql: &str, i: usize) -> usize {
    sql[i..].find(['\n', '\r']).map_or(sql.len(), |n| i + n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dash_comments() {
        for (sql, want) in [
            ("VALUES 1", "VALUES 1"),
            ("VALUES 1 -- one", "VALUES 1 "),
            ("VALUES 1--", "VALUES 1"),
            ("-- a\nVALUES 1 -- b\r\n-- c", "\nVALUES 1 \r\n"),
            ("VALUES 1 - -1", "VALUES 1 - -1"),
            (
                "VALUES '--', 'it''s -- x' -- y",
                "VALUES '--', 'it''s -- x' ",
            ),
            (r#"SELECT 1 AS "a--b" -- c"#, r#"SELECT 1 AS "a--b" "#),
            ("VALUES 1 /* -- \n */ -- x", "VALUES 1 /* -- \n */ "),
            ("VALUES 'é--' -- é", "VALUES 'é--' "),
            ("VALUES é -- é", "VALUES é "),
            // Not closed, so they're left for libvsql to report.
            ("VALUES '-- x", "VALUES '-- x"),
            ("VALUES 1 /* -- x", "VALUES 1 /* -- x"),
        ] {
            assert_eq!(remove_dash_comments(sql), want, "{sql}");
        }
    }

    #[test]
    fn statements() {
        let empty: [&str; 0] = [];
        for (sql, want) in [
            ("", &empty[..]),
            (" ; \n;", &empty),
            ("-- a; b\n/* c; */ ;", &empty),
            ("VALUES 1", &["VALUES 1"]),
            ("VALUES 1;", &["VALUES 1"]),
            ("VALUES 1; VALUES 2", &["VALUES 1", " VALUES 2"]),
            ("VALUES 1;;\nVALUES 2;\n", &["VALUES 1", "\nVALUES 2"]),
            ("VALUES ';', 'it''s;'; x", &["VALUES ';', 'it''s;'", " x"]),
            (r#"SELECT 1 AS "a;b"; x"#, &[r#"SELECT 1 AS "a;b""#, " x"]),
            ("VALUES 1 -- a; b\n; x", &["VALUES 1 -- a; b\n", " x"]),
            ("VALUES 1 /* a; b */; x", &["VALUES 1 /* a; b */", " x"]),
            ("'a'; é; x", &["'a'", " é", " x"]),
            // Not closed, so they're left for libvsql to report.
            ("VALUES 1; VALUES ';", &["VALUES 1", " VALUES ';"]),
            ("VALUES 1; /* ;", &["VALUES 1", " /* ;"]),
        ] {
            assert_eq!(split_statements(sql), want, "{sql}");
        }
    }
}
