/// Cloud queue metric views (`pendingSize`/`delayedSize`/`reservedSize`/oldest).
use serde::{Deserialize, Serialize};

/// Per-queue depth metrics — the four Cloud queue gauges per queue.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueMetrics {
    /// Connection (driver) name this row belongs to.
    pub connection: String,
    /// Queue name this row describes.
    pub queue: String,
    /// Number of available jobs (`pendingSize`).
    pub pending: usize,
    /// Number of delayed jobs not yet available (`delayedSize`).
    pub delayed: usize,
    /// Number of reserved in-flight jobs (`reservedSize`).
    pub reserved: usize,
    /// UTC instant of the oldest pending job; `None` on an empty queue.
    pub oldest_pending: Option<chrono::DateTime<chrono::Utc>>,
}

/// Named queue family with metric rows per `(connection, queue)` pair.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Queues {
    /// Per-queue rows keyed by `(connection, queue)`.
    pub queues: Vec<QueueMetrics>,
}

impl Queues {
    /// Metric row for a single `(connection, queue)` pair.
    pub fn row(&self, connection: &str, queue: &str) -> Option<&QueueMetrics> {
        self.queues
            .iter()
            .find(|q| q.connection == connection && q.queue == queue)
    }

    /// Total pending depth across all queues.
    pub fn pending(&self) -> usize {
        self.queues.iter().map(|q| q.pending).sum()
    }

    /// Total delayed depth across all queues.
    pub fn delayed(&self) -> usize {
        self.queues.iter().map(|q| q.delayed).sum()
    }

    /// Total reserved depth across all queues.
    pub fn reserved(&self) -> usize {
        self.queues.iter().map(|q| q.reserved).sum()
    }

    /// Earliest oldest-pending instant across all queues (`None` when none).
    pub fn oldest_pending(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.queues.iter().filter_map(|q| q.oldest_pending).min()
    }

    /// Register or update the metric row for a `(connection, queue)` pair.
    ///
    /// Keyed on both fields so two connections sharing a queue name keep
    /// distinct rows instead of one overwriting the other.
    pub fn upsert(&mut self, row: QueueMetrics) {
        if let Some(existing) = self
            .queues
            .iter_mut()
            .find(|q| q.connection == row.connection && q.queue == row.queue)
        {
            *existing = row;
        } else {
            self.queues.push(row);
        }
    }
}

/// Convenience metric holder used by drivers during snapshots.
#[derive(Debug, Default)]
pub struct JobQueueMetrics {
    /// Queues aggregated under this holder.
    pub queues: Queues,
}

impl JobQueueMetrics {
    /// Record a row into the holder.
    pub fn record(&mut self, row: QueueMetrics) {
        self.queues.upsert(row);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    /// Build a metric row for `(connection, queue)` with the given oldest instant.
    fn row(
        connection: &str,
        queue: &str,
        pending: usize,
        oldest: Option<chrono::DateTime<Utc>>,
    ) -> QueueMetrics {
        QueueMetrics {
            connection: connection.to_string(),
            queue: queue.to_string(),
            pending,
            delayed: 0,
            reserved: 0,
            oldest_pending: oldest,
        }
    }

    /// `oldest_pending` returns the minimum instant across all rows.
    #[test]
    fn oldest_pending_is_minimum_across_rows() {
        let early = Utc.timestamp_opt(1_700_000_000, 0).single().expect("early");
        let late = Utc.timestamp_opt(1_700_000_100, 0).single().expect("late");
        let mut queues = Queues::default();
        queues.upsert(row("sync", "a", 1, Some(late)));
        queues.upsert(row("sync", "b", 1, Some(early)));
        queues.upsert(row("sync", "c", 0, None));
        assert_eq!(queues.oldest_pending(), Some(early));
        assert_eq!(queues.pending(), 2);
    }

    /// A snapshot with no dated rows has no oldest instant.
    #[test]
    fn oldest_pending_none_when_all_empty() {
        let mut queues = Queues::default();
        queues.upsert(row("sync", "a", 0, None));
        assert_eq!(queues.oldest_pending(), None);
    }

    /// The same queue name on two connections yields two distinct rows.
    #[test]
    fn upsert_keys_rows_by_connection_and_queue() {
        let mut queues = Queues::default();
        queues.upsert(row("redis", "default", 1, None));
        queues.upsert(row("database", "default", 2, None));
        assert_eq!(queues.queues.len(), 2, "one row per (connection, queue)");
        assert_eq!(queues.row("redis", "default").expect("redis").pending, 1);
        assert_eq!(
            queues.row("database", "default").expect("database").pending,
            2
        );
    }

    /// Upserting the same `(connection, queue)` replaces the existing row.
    #[test]
    fn upsert_replaces_same_connection_and_queue() {
        let mut queues = Queues::default();
        queues.upsert(row("redis", "default", 1, None));
        queues.upsert(row("redis", "default", 5, None));
        assert_eq!(queues.queues.len(), 1);
        assert_eq!(queues.row("redis", "default").expect("row").pending, 5);
    }

    /// No targets means an empty snapshot with no matching rows.
    #[test]
    fn empty_targets_yield_empty_queues() {
        let queues = Queues::default();
        assert!(queues.queues.is_empty());
        assert_eq!(queues.row("redis", "default"), None);
        assert_eq!(queues.pending(), 0);
        assert_eq!(queues.oldest_pending(), None);
    }
}
