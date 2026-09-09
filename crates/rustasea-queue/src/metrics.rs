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
