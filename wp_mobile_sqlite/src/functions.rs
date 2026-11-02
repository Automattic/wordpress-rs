use rusqlite::Connection;
use rusqlite::Result as SqliteResult;
use rusqlite::functions::FunctionFlags;
use wp_api::parsed_url::ParsedUrl;

/// Register the `urls_eq(a, b)` SQL function on `conn`.
///
/// `urls_eq` is true when `a` and `b` are the same string, or when both parse
/// to equal `ParsedUrl`s, such as `https://example.com` and
/// `https://example.com/`. A query can use it to find a stored URL written in a
/// different form than the one it looks up, without migrating stored values.
pub(crate) fn register_url_functions(conn: &Connection) -> SqliteResult<()> {
    conn.create_scalar_function(
        "urls_eq",
        2,
        FunctionFlags::SQLITE_DETERMINISTIC | FunctionFlags::SQLITE_UTF8,
        |ctx| {
            let a: String = ctx.get(0)?;
            let b: String = ctx.get(1)?;
            if a == b {
                return Ok(true);
            }
            Ok(match (ParsedUrl::parse(&a), ParsedUrl::parse(&b)) {
                (Ok(parsed_a), Ok(parsed_b)) => parsed_a == parsed_b,
                _ => false,
            })
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_urls_eq_matches_across_trailing_slash() {
        let conn = Connection::open_in_memory().unwrap();
        register_url_functions(&conn).unwrap();

        for (a, b) in [
            ("https://example.com", "https://example.com/"),
            ("https://example.com/", "https://example.com"),
            ("https://example.com", "https://example.com"),
        ] {
            let result: bool = conn
                .query_row("SELECT urls_eq(?, ?)", [a, b], |row| row.get(0))
                .unwrap();
            assert!(result, "{a:?} should equal {b:?}");
        }
    }

    #[test]
    fn test_urls_eq_is_case_insensitive_in_scheme_and_host() {
        let conn = Connection::open_in_memory().unwrap();
        register_url_functions(&conn).unwrap();

        let result: bool = conn
            .query_row(
                "SELECT urls_eq(?, ?)",
                ["http://localhost", "HTTP://LOCALHOST"],
                |row| row.get(0),
            )
            .unwrap();
        assert!(result);
    }

    #[test]
    fn test_urls_eq_returns_false_for_different_hosts() {
        let conn = Connection::open_in_memory().unwrap();
        register_url_functions(&conn).unwrap();

        let result: bool = conn
            .query_row(
                "SELECT urls_eq(?, ?)",
                ["https://example.com", "https://other.example.com"],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!result);
    }

    #[test]
    fn test_urls_eq_falls_back_to_string_equality_for_unparseable_input() {
        let conn = Connection::open_in_memory().unwrap();
        register_url_functions(&conn).unwrap();

        let same: bool = conn
            .query_row("SELECT urls_eq(?, ?)", ["not a url", "not a url"], |row| {
                row.get(0)
            })
            .unwrap();
        assert!(same, "identical unparseable strings should match");

        let different: bool = conn
            .query_row(
                "SELECT urls_eq(?, ?)",
                ["not a url", "also not a url"],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!different, "distinct unparseable strings should not match");
    }
}
