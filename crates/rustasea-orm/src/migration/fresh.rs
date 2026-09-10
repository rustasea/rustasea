//! `migrate:fresh` table teardown, split out of [`crate::migration`].
//!
//! Keeps the migration runner module within the file-size limit. Catalog table
//! names are escaped before interpolation and, on MySQL, every statement runs on
//! one pooled connection so the FK-check toggle is session-consistent.

use crate::db::DbPool;
use crate::error::{OrmError, Result};

/// Drop every user table for the pool's dialect (`migrate:fresh`).
///
/// Catalog identifiers are escaped before interpolation — SQLite doubles `"` and
/// MySQL doubles `` ` `` — so a table named `x"; DROP TABLE users; --` cannot
/// break out of its quoting. On MySQL a single pooled connection runs
/// `SET FOREIGN_KEY_CHECKS = 0`, every drop, then the re-enable, so the toggle
/// applies to the drops and the session always leaves with FK checks enabled.
pub(crate) async fn drop_all_tables(pool: &DbPool) -> Result<()> {
    match pool.dialect() {
        "sqlite" => {
            let rows = pool
                .fetch_json(
                    "SELECT name FROM sqlite_master \
                     WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
                    &[],
                )
                .await?;
            for name in table_names(&rows) {
                let escaped = escape_identifier(&name, '"');
                pool.execute_script(&format!("DROP TABLE IF EXISTS \"{escaped}\";"))
                    .await?;
            }
        }
        "postgres" => {
            pool.execute_script("DROP SCHEMA public CASCADE; CREATE SCHEMA public;")
                .await?;
        }
        "mysql" => {
            #[cfg(feature = "mysql")]
            {
                drop_mysql_tables(pool).await?;
            }
            #[cfg(not(feature = "mysql"))]
            {
                return Err(OrmError::UnsupportedDriver("mysql".to_string()));
            }
        }
        other => {
            return Err(OrmError::UnsupportedDriver(other.to_string()));
        }
    }
    Ok(())
}

/// Drop every MySQL table on one pooled connection with FK checks disabled.
///
/// The re-enable always runs on that same session, even after a failed drop, so
/// a returned connection never leaks disabled FK checks into the pool.
#[cfg(feature = "mysql")]
async fn drop_mysql_tables(pool: &DbPool) -> Result<()> {
    use sqlx::Executor as _;

    let inner = match pool {
        DbPool::MySql(pool) => pool,
        _ => return Err(OrmError::UnsupportedDriver(pool.dialect().to_string())),
    };
    let mut conn = inner.acquire().await?;
    (&mut *conn)
        .execute(sqlx::raw_sql("SET FOREIGN_KEY_CHECKS = 0;"))
        .await?;
    let dropped = drop_mysql_tables_on(&mut conn).await;
    let restored = (&mut *conn)
        .execute(sqlx::raw_sql("SET FOREIGN_KEY_CHECKS = 1;"))
        .await
        .map(|_| ());
    match (dropped, restored) {
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(error.into()),
        (Ok(()), Ok(())) => Ok(()),
    }
}

/// Fetch and drop every table in the current MySQL schema on `conn`.
#[cfg(feature = "mysql")]
async fn drop_mysql_tables_on(conn: &mut sqlx::MySqlConnection) -> Result<()> {
    use sqlx::{Executor as _, Row as _};

    let rows = sqlx::query(
        "SELECT table_name AS name FROM information_schema.tables \
         WHERE table_schema = DATABASE()",
    )
    .fetch_all(&mut *conn)
    .await?;
    for row in &rows {
        let name: String = row.try_get("name")?;
        let escaped = escape_identifier(&name, '`');
        let sql = format!("DROP TABLE IF EXISTS `{escaped}`;");
        (&mut *conn).execute(sqlx::raw_sql(&sql)).await?;
    }
    Ok(())
}

/// Escape a SQL identifier by doubling embedded occurrences of `quote`.
fn escape_identifier(name: &str, quote: char) -> String {
    let doubled = format!("{quote}{quote}");
    name.replace(quote, &doubled)
}

/// Extract `name` values from JSON rows (missing/null names are skipped).
fn table_names(rows: &[serde_json::Value]) -> Vec<String> {
    rows.iter()
        .filter_map(|row| row.get("name").and_then(|n| n.as_str()).map(str::to_string))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies SQLite-style double quotes are doubled.
    #[test]
    fn escape_doubles_double_quotes() {
        assert_eq!(
            escape_identifier(r#"x"; DROP TABLE users; --"#, '"'),
            r#"x""; DROP TABLE users; --"#
        );
    }

    /// Verifies MySQL-style backticks are doubled.
    #[test]
    fn escape_doubles_backticks() {
        assert_eq!(
            escape_identifier("x`; DROP TABLE users; --", '`'),
            "x``; DROP TABLE users; --"
        );
    }

    /// Verifies ordinary identifiers pass through untouched.
    #[test]
    fn escape_leaves_plain_identifiers() {
        assert_eq!(escape_identifier("users", '"'), "users");
        assert_eq!(escape_identifier("users", '`'), "users");
    }

    /// Verifies `table_names` skips missing and null names.
    #[test]
    fn table_names_skips_missing_and_null() {
        let rows = vec![
            serde_json::json!({ "name": "users" }),
            serde_json::json!({ "name": null }),
            serde_json::json!({ "other": "ignored" }),
            serde_json::json!({ "name": "posts" }),
        ];
        assert_eq!(table_names(&rows), vec!["users", "posts"]);
    }
}
