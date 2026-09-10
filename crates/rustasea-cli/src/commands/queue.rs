//! Queue worker command — `queue:work` (M4).
//!
//! Opens a real database pool from config, installs a `DatabaseDriver` under
//! the canonical `database` connection, and drains the requested queues through
//! the queue worker loop. Handlers are resolved from the process-wide registry
//! populated by `rustasea_queue::register_job::<T>()` at application boot.

use async_trait::async_trait;
use rustasea_orm::DbPool;
use rustasea_queue::QueueDriver;
use std::sync::Arc;

use crate::artisan::{Command, Io};
use crate::error::{CliError, CliResult};
use crate::commands::ops::database_url;

/// `queue:work` — process jobs from the database queue.
pub struct QueueWork;

#[async_trait]
impl Command for QueueWork {
    /// Command signature.
    fn signature(&self) -> &'static str {
        "queue:work"
    }

    /// Usage line rendered by `list`.
    fn usage(&self) -> Option<&'static str> {
        Some("queue:work [--queue=default] [--max-jobs=1]")
    }

    /// One-line help rendered by `list`.
    fn help(&self) -> Option<&'static str> {
        Some("Process jobs from the database queue")
    }

    /// Execute: open a pool, register the driver, and drain the queue(s).
    async fn run(&self, args: Vec<String>, io: &mut Io) -> CliResult<()> {
        let queue = option_value(&args, "--queue").unwrap_or_else(|| "default".to_string());
        let max_jobs = option_value(&args, "--max-jobs")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(1);

        let url = database_url()?;
        let pool = DbPool::connect(&url)
            .await
            .map_err(|error| CliError::Domain(error.to_string()))?;
        // Install through the shared registry path so dispatches to the
        // `database` connection resolve to this driver too.
        let driver = rustasea_queue::register_database_driver(pool.clone());

        let processed = rustasea_queue::run_worker_with(
            driver.clone() as Arc<dyn QueueDriver>,
            vec![queue.clone()],
            max_jobs,
            rustasea_queue::default_resolver(),
        )
        .await
        .map_err(|error| CliError::Domain(error.to_string()))?;

        io.line(format!(
            "Processed {processed} job(s) from queue `{queue}`."
        ));
        pool.close().await;
        Ok(())
    }
}

/// Read a `--key=value` option from the argument list.
fn option_value(args: &[String], key: &str) -> Option<String> {
    let prefix = format!("{key}=");
    args.iter()
        .find_map(|arg| arg.strip_prefix(&prefix).map(str::to_string))
}
