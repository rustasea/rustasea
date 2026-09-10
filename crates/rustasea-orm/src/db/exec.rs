//! Driver-level execution over a [`DbPool`].
//!
//! [`DbPool::fetch_json`] runs a parameterized `SELECT` and decodes every row to
//! a JSON object; [`DbPool::execute_bind`] runs a statement and returns the
//! affected row count. Both use the `sqlx` runtime API only — no compile-time
//! `query!` macros — and dispatch to the concrete pool for the active driver.

use crate::db::DbPool;
#[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
use crate::error::OrmError;
use crate::error::Result;
use crate::tx::DbTransaction;
use crate::types::Value;
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use crate::value::{bind_all, row_to_json};

/// Translate `$n` placeholders in `sql` to the given driver's native shape.
///
/// All emitters (`QueryBuilder`, `model_ops`, `m2`) produce `$n` positional
/// placeholders; SQLite and Postgres accept them natively, while MySQL requires
/// `?`. Rewriting here keeps the emitters dialect-agnostic and makes this the
/// single choke point for placeholder adaptation. Only SQL text is rewritten —
/// bind values are never inspected, so a literal `$` inside a bound string is
/// unaffected.
fn adapt_placeholders<'a>(sql: &'a str, dialect: &str) -> std::borrow::Cow<'a, str> {
    #[cfg(feature = "mysql")]
    {
        if dialect == "mysql" {
            return rewrite_dollar_placeholders(sql);
        }
    }
    let _ = dialect;
    std::borrow::Cow::Borrowed(sql)
}

/// Rewrite `$n` references to `?`, renumbering sequentially.
///
/// MySQL's `?` placeholders are positional, not indexed, so non-contiguous `$n`
/// indices (e.g. `$1 … $3`) collapse to a sequential run of `?` in textual
/// order. `$` not followed by one or more digits is left untouched, which
/// preserves dollar signs appearing as literals in the SQL text.
#[cfg(feature = "mysql")]
fn rewrite_dollar_placeholders(sql: &str) -> std::borrow::Cow<'_, str> {
    let bytes = sql.as_bytes();
    let mut out = String::with_capacity(sql.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'$' && index + 1 < bytes.len() && bytes[index + 1].is_ascii_digit() {
            out.push('?');
            index += 1;
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                index += 1;
            }
        } else {
            // Copy one full UTF-8 scalar; multi-byte bytes are never `$`/digits.
            let ch = sql[index..].chars().next().unwrap_or_default();
            out.push(ch);
            index += ch.len_utf8();
        }
    }
    std::borrow::Cow::Owned(out)
}

impl DbPool {
    /// Run `sql` with `bindings` and decode every row into a JSON object.
    ///
    /// The statement must use `$n` positional placeholders matching `bindings`;
    /// [`adapt_placeholders`] rewrites them to the active driver's native shape
    /// before dispatch.
    pub async fn fetch_json(
        &self,
        _sql: &str,
        _bindings: &[Value],
    ) -> Result<Vec<serde_json::Value>> {
        let sql = adapt_placeholders(_sql, self.dialect());
        match self {
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => fetch_json_sqlite(pool, &sql, _bindings).await,
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => fetch_json_postgres(pool, &sql, _bindings).await,
            #[cfg(feature = "mysql")]
            DbPool::MySql(pool) => fetch_json_mysql(pool, &sql, _bindings).await,
            #[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
            _ => Err(OrmError::UnsupportedDriver(
                "no sqlx driver compiled in".into(),
            )),
        }
    }

    /// Run `sql` with `bindings` and return the number of affected rows.
    ///
    /// The statement must use `$n` positional placeholders matching `bindings`;
    /// [`adapt_placeholders`] rewrites them to the active driver's native shape
    /// before dispatch.
    pub async fn execute_bind(&self, _sql: &str, _bindings: &[Value]) -> Result<u64> {
        let sql = adapt_placeholders(_sql, self.dialect());
        match self {
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => execute_sqlite(pool, &sql, _bindings).await,
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => execute_postgres(pool, &sql, _bindings).await,
            #[cfg(feature = "mysql")]
            DbPool::MySql(pool) => execute_mysql(pool, &sql, _bindings).await,
            #[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
            _ => Err(OrmError::UnsupportedDriver(
                "no sqlx driver compiled in".into(),
            )),
        }
    }

    /// Execute a `;`-separated SQL script with no bind values.
    ///
    /// Used by migrations and seeders whose bodies contain multiple statements
    /// (`sqlx::query` accepts one statement only). The script is sent verbatim —
    /// never interpolate user input; pass it as a bound value instead.
    pub async fn execute_script(&self, _sql: &str) -> Result<()> {
        match self {
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::raw_sql(_sql).execute(pool).await?;
            }
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::raw_sql(_sql).execute(pool).await?;
            }
            #[cfg(feature = "mysql")]
            DbPool::MySql(pool) => {
                sqlx::raw_sql(_sql).execute(pool).await?;
            }
            #[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
            _ => {
                return Err(OrmError::UnsupportedDriver(
                    "no sqlx driver compiled in".into(),
                ))
            }
        }
        Ok(())
    }
}

impl DbTransaction {
    /// The dialect name for this transaction (`sqlite` / `postgres` / `mysql`).
    pub(crate) fn dialect(&self) -> &'static str {
        match self {
            #[cfg(feature = "sqlite")]
            DbTransaction::Sqlite(_) => "sqlite",
            #[cfg(feature = "postgres")]
            DbTransaction::Postgres(_) => "postgres",
            #[cfg(feature = "mysql")]
            DbTransaction::MySql(_) => "mysql",
            #[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
            _ => "unknown",
        }
    }

    /// Run `sql` with `bindings` on the transaction, decoding every row to JSON.
    pub(crate) async fn fetch_json(
        &mut self,
        sql: &str,
        bindings: &[Value],
    ) -> Result<Vec<serde_json::Value>> {
        let sql = adapt_placeholders(sql, self.dialect());
        match self {
            #[cfg(feature = "sqlite")]
            DbTransaction::Sqlite(tx) => fetch_json_sqlite_tx(tx, &sql, bindings).await,
            #[cfg(feature = "postgres")]
            DbTransaction::Postgres(tx) => fetch_json_postgres_tx(tx, &sql, bindings).await,
            #[cfg(feature = "mysql")]
            DbTransaction::MySql(tx) => fetch_json_mysql_tx(tx, &sql, bindings).await,
            #[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
            _ => Err(OrmError::UnsupportedDriver(
                "no sqlx driver compiled in".into(),
            )),
        }
    }

    /// Run `sql` with `bindings` on the transaction, returning affected rows.
    pub(crate) async fn execute_bind(&mut self, sql: &str, bindings: &[Value]) -> Result<u64> {
        let sql = adapt_placeholders(sql, self.dialect());
        match self {
            #[cfg(feature = "sqlite")]
            DbTransaction::Sqlite(tx) => execute_sqlite_tx(tx, &sql, bindings).await,
            #[cfg(feature = "postgres")]
            DbTransaction::Postgres(tx) => execute_postgres_tx(tx, &sql, bindings).await,
            #[cfg(feature = "mysql")]
            DbTransaction::MySql(tx) => execute_mysql_tx(tx, &sql, bindings).await,
            #[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
            _ => Err(OrmError::UnsupportedDriver(
                "no sqlx driver compiled in".into(),
            )),
        }
    }

    /// Execute a `;`-separated SQL script on the transaction, with no bind values.
    ///
    /// The future is boxed per driver arm so the concrete `Send` future is
    /// visible to higher-ranked transaction closures (the generic `raw_sql`
    /// future is otherwise not provably `Send` for every lifetime).
    pub(crate) fn execute_script<'a>(
        &'a mut self,
        sql: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        match self {
            #[cfg(feature = "sqlite")]
            DbTransaction::Sqlite(tx) => Box::pin(execute_sqlite_script_tx(tx, sql)),
            #[cfg(feature = "postgres")]
            DbTransaction::Postgres(tx) => Box::pin(execute_postgres_script_tx(tx, sql)),
            #[cfg(feature = "mysql")]
            DbTransaction::MySql(tx) => Box::pin(execute_mysql_script_tx(tx, sql)),
            #[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
            _ => Box::pin(async move {
                let _ = sql;
                Err(OrmError::UnsupportedDriver(
                    "no sqlx driver compiled in".into(),
                ))
            }),
        }
    }
}

/// Fetch rows from an open SQLite transaction as JSON objects.
#[cfg(feature = "sqlite")]
async fn fetch_json_sqlite_tx(
    tx: &mut sqlx::Transaction<'static, sqlx::Sqlite>,
    sql: &str,
    bindings: &[Value],
) -> Result<Vec<serde_json::Value>> {
    let query = bind_all::<sqlx::Sqlite>(sqlx::query(sql), bindings);
    let rows = query.fetch_all(&mut **tx).await?;
    rows.iter().map(row_to_json::<sqlx::Sqlite>).collect()
}

/// Execute a statement against a SQLite transaction, returning affected rows.
#[cfg(feature = "sqlite")]
async fn execute_sqlite_tx(
    tx: &mut sqlx::Transaction<'static, sqlx::Sqlite>,
    sql: &str,
    bindings: &[Value],
) -> Result<u64> {
    let query = bind_all::<sqlx::Sqlite>(sqlx::query(sql), bindings);
    Ok(query.execute(&mut **tx).await?.rows_affected())
}

/// Execute a `;`-separated script against a SQLite transaction.
#[cfg(feature = "sqlite")]
async fn execute_sqlite_script_tx(
    tx: &mut sqlx::Transaction<'static, sqlx::Sqlite>,
    sql: &str,
) -> Result<()> {
    use sqlx::Executor as _;
    (&mut **tx).execute(sqlx::raw_sql(sql)).await?;
    Ok(())
}

/// Fetch rows from an open Postgres transaction as JSON objects.
#[cfg(feature = "postgres")]
async fn fetch_json_postgres_tx(
    tx: &mut sqlx::Transaction<'static, sqlx::Postgres>,
    sql: &str,
    bindings: &[Value],
) -> Result<Vec<serde_json::Value>> {
    let query = bind_all::<sqlx::Postgres>(sqlx::query(sql), bindings);
    let rows = query.fetch_all(&mut **tx).await?;
    rows.iter().map(row_to_json::<sqlx::Postgres>).collect()
}

/// Execute a statement against a Postgres transaction, returning affected rows.
#[cfg(feature = "postgres")]
async fn execute_postgres_tx(
    tx: &mut sqlx::Transaction<'static, sqlx::Postgres>,
    sql: &str,
    bindings: &[Value],
) -> Result<u64> {
    let query = bind_all::<sqlx::Postgres>(sqlx::query(sql), bindings);
    Ok(query.execute(&mut **tx).await?.rows_affected())
}

/// Execute a `;`-separated script against a Postgres transaction.
#[cfg(feature = "postgres")]
async fn execute_postgres_script_tx(
    tx: &mut sqlx::Transaction<'static, sqlx::Postgres>,
    sql: &str,
) -> Result<()> {
    use sqlx::Executor as _;
    (&mut **tx).execute(sqlx::raw_sql(sql)).await?;
    Ok(())
}

/// Fetch rows from an open MySQL transaction as JSON objects.
#[cfg(feature = "mysql")]
async fn fetch_json_mysql_tx(
    tx: &mut sqlx::Transaction<'static, sqlx::MySql>,
    sql: &str,
    bindings: &[Value],
) -> Result<Vec<serde_json::Value>> {
    let query = bind_all::<sqlx::MySql>(sqlx::query(sql), bindings);
    let rows = query.fetch_all(&mut **tx).await?;
    rows.iter().map(row_to_json::<sqlx::MySql>).collect()
}

/// Execute a statement against a MySQL transaction, returning affected rows.
#[cfg(feature = "mysql")]
async fn execute_mysql_tx(
    tx: &mut sqlx::Transaction<'static, sqlx::MySql>,
    sql: &str,
    bindings: &[Value],
) -> Result<u64> {
    let query = bind_all::<sqlx::MySql>(sqlx::query(sql), bindings);
    Ok(query.execute(&mut **tx).await?.rows_affected())
}

/// Execute a `;`-separated script against a MySQL transaction.
#[cfg(feature = "mysql")]
async fn execute_mysql_script_tx(
    tx: &mut sqlx::Transaction<'static, sqlx::MySql>,
    sql: &str,
) -> Result<()> {
    use sqlx::Executor as _;
    (&mut **tx).execute(sqlx::raw_sql(sql)).await?;
    Ok(())
}

/// Fetch rows from a SQLite pool as JSON objects.
#[cfg(feature = "sqlite")]
async fn fetch_json_sqlite(
    pool: &sqlx::SqlitePool,
    sql: &str,
    bindings: &[Value],
) -> Result<Vec<serde_json::Value>> {
    let query = bind_all::<sqlx::Sqlite>(sqlx::query(sql), bindings);
    let rows = query.fetch_all(pool).await?;
    rows.iter().map(row_to_json::<sqlx::Sqlite>).collect()
}

/// Execute a statement against a SQLite pool, returning affected rows.
#[cfg(feature = "sqlite")]
async fn execute_sqlite(pool: &sqlx::SqlitePool, sql: &str, bindings: &[Value]) -> Result<u64> {
    let query = bind_all::<sqlx::Sqlite>(sqlx::query(sql), bindings);
    Ok(query.execute(pool).await?.rows_affected())
}

/// Fetch rows from a Postgres pool as JSON objects.
#[cfg(feature = "postgres")]
async fn fetch_json_postgres(
    pool: &sqlx::PgPool,
    sql: &str,
    bindings: &[Value],
) -> Result<Vec<serde_json::Value>> {
    let query = bind_all::<sqlx::Postgres>(sqlx::query(sql), bindings);
    let rows = query.fetch_all(pool).await?;
    rows.iter().map(row_to_json::<sqlx::Postgres>).collect()
}

/// Execute a statement against a Postgres pool, returning affected rows.
#[cfg(feature = "postgres")]
async fn execute_postgres(pool: &sqlx::PgPool, sql: &str, bindings: &[Value]) -> Result<u64> {
    let query = bind_all::<sqlx::Postgres>(sqlx::query(sql), bindings);
    Ok(query.execute(pool).await?.rows_affected())
}

/// Fetch rows from a MySQL pool as JSON objects.
#[cfg(feature = "mysql")]
async fn fetch_json_mysql(
    pool: &sqlx::MySqlPool,
    sql: &str,
    bindings: &[Value],
) -> Result<Vec<serde_json::Value>> {
    let query = bind_all::<sqlx::MySql>(sqlx::query(sql), bindings);
    let rows = query.fetch_all(pool).await?;
    rows.iter().map(row_to_json::<sqlx::MySql>).collect()
}

/// Execute a statement against a MySQL pool, returning affected rows.
#[cfg(feature = "mysql")]
async fn execute_mysql(pool: &sqlx::MySqlPool, sql: &str, bindings: &[Value]) -> Result<u64> {
    let query = bind_all::<sqlx::MySql>(sqlx::query(sql), bindings);
    Ok(query.execute(pool).await?.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies MySQL rewrites sequential `$n` placeholders to `?` in order.
    #[cfg(feature = "mysql")]
    #[test]
    fn mysql_rewrites_sequential_placeholders_in_order() {
        let adapted =
            adapt_placeholders("SELECT * FROM users WHERE id = $1 AND email = $2", "mysql");
        assert_eq!(
            adapted.as_ref(),
            "SELECT * FROM users WHERE id = ? AND email = ?"
        );
    }

    /// Verifies non-contiguous `$n` indices collapse to sequential `?`.
    #[cfg(feature = "mysql")]
    #[test]
    fn mysql_renumbers_non_contiguous_placeholders() {
        let adapted = adapt_placeholders("UPDATE t SET a = $1, b = $3 WHERE id = $1", "mysql");
        assert_eq!(adapted.as_ref(), "UPDATE t SET a = ?, b = ? WHERE id = ?");
    }

    /// Verifies a `$` not followed by digits in SQL text is left untouched.
    #[cfg(feature = "mysql")]
    #[test]
    fn mysql_leaves_non_placeholder_dollar_untouched() {
        let adapted = adapt_placeholders(
            "SELECT '$abc' AS tag, 'a$' AS tail FROM t WHERE id = $1",
            "mysql",
        );
        assert_eq!(
            adapted.as_ref(),
            "SELECT '$abc' AS tag, 'a$' AS tail FROM t WHERE id = ?"
        );
    }

    /// Verifies translation operates on SQL text only — bind values keep `$`.
    #[cfg(feature = "mysql")]
    #[test]
    fn mysql_does_not_touch_bind_values_containing_dollar() {
        let bindings = vec![Value::Text("$2 literal".into())];
        let adapted = adapt_placeholders("SELECT * FROM t WHERE note = $1", "mysql");
        assert_eq!(adapted.as_ref(), "SELECT * FROM t WHERE note = ?");
        assert_eq!(bindings[0], Value::Text("$2 literal".into()));
    }

    /// Verifies double-digit placeholders renumber without leaving residue.
    #[cfg(feature = "mysql")]
    #[test]
    fn mysql_renumbers_double_digit_placeholders() {
        let placeholders: Vec<String> = (1..=12).map(|i| format!("${i}")).collect();
        let sql = format!("SELECT {}", placeholders.join(", "));
        let adapted = adapt_placeholders(&sql, "mysql");
        let expected = vec!["?"; 12].join(", ");
        assert_eq!(adapted.as_ref(), format!("SELECT {expected}"));
    }

    /// Verifies SQLite/Postgres SQL passes through unchanged (`$n` is native).
    #[test]
    fn non_mysql_dialects_keep_dollar_placeholders() {
        let sql = "SELECT * FROM users WHERE id = $1 AND email = $2";
        for dialect in ["sqlite", "postgres"] {
            let adapted = adapt_placeholders(sql, dialect);
            assert_eq!(adapted.as_ref(), sql, "dialect {dialect} must pass through");
        }
    }
}
