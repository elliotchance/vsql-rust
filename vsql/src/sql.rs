// Changes to the text of a statement before it's given to libvsql.

use std::borrow::Cow;

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
        match bytes[i] {
            // A string or a quoted identifier, where a quote is escaped by
            // doubling it (which is the same as two strings in a row).
            quote @ (b'\'' | b'"') => {
                i += 1;
                while i < bytes.len() && bytes[i] != quote {
                    i += 1;
                }
                i += 1;
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i = match sql[i + 2..].find("*/") {
                    Some(end) => i + 2 + end + 2,
                    None => bytes.len(),
                };
            }
            b'-' if bytes.get(i + 1) == Some(&b'-') => {
                out.push_str(&sql[start..i]);
                i = match sql[i..].find(['\n', '\r']) {
                    Some(end) => i + end,
                    None => bytes.len(),
                };
                start = i;
            }
            _ => i += 1,
        }
    }
    out.push_str(&sql[start.min(sql.len())..]);

    Cow::Owned(out)
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
            // Not closed, so they're left for libvsql to report.
            ("VALUES '-- x", "VALUES '-- x"),
            ("VALUES 1 /* -- x", "VALUES 1 /* -- x"),
        ] {
            assert_eq!(remove_dash_comments(sql), want, "{sql}");
        }
    }
}
