/// Cloud queue metric views (`pendingSize`/`delayedSize`/`reservedSize`/oldest).
use serde::{Deserialize, Serialize};

/// Per-queue depth metrics — the four Cloud queue gauges per queue.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueMetrics {
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

/// Named queue family with metric rows per queue.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Queues {
    /// Per-queue rows keyed by queue name.
    pub queues: Vec<QueueMetrics>,
}

impl Queues {
    /// Metric row for a single queue.
    pub fn row(&self, queue: &str) -> Option<&QueueMetrics> {
        self.queues.iter().find(|q| q.queue == queue)
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

    /// Register or update the metric row for `queue`.
    pub fn upsert(&mut self, row: QueueMetrics) {
        if let Some(existing) = self.queues.iter_mut().find(|q| q.queue == row.queue) {
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

    /// Build a metric row for `queue` with the given oldest instant.
    fn row(queue: &str, pending: usize, oldest: Option<chrono::DateTime<Utc>>) -> QueueMetrics {
        QueueMetrics {
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
        queues.upsert(row("a", 1, Some(late)));
        queues.upsert(row("b", 1, Some(early)));
        queues.upsert(row("c", 0, None));
        assert_eq!(queues.oldest_pending(), Some(early));
        assert_eq!(queues.pending(), 2);
    }

    /// A snapshot with no dated rows has no oldest instant.
    #[test]
    fn oldest_pending_none_when_all_empty() {
        let mut queues = Queues::default();
        queues.upsert(row("a", 0, None));
        assert_eq!(queues.oldest_pending(), None);
    }
}
