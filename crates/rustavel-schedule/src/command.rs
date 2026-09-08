/// A registered scheduled command with its frequency and modifiers.
use std::time::Duration;

use chrono::{Datelike, Timelike};
use serde::{Deserialize, Serialize};

use crate::error::{Result, ScheduleError};

/// One scheduled command entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleCommand {
    /// Command name (e.g. `emails:send`).
    pub command: &'static str,
    /// Cron expression (5-field).
    pub cron: String,
    /// Human-readable frequency description for `schedule:list`.
    pub frequency: String,
    /// Whether overlapping runs are suppressed.
    pub skip_if_running: bool,
    /// Whether only one server may run this command.
    pub on_one_server: bool,
    /// Minimum delay between ticks (tick pacing).
    pub every: Duration,
    /// Lock key used by `onOneServer` (derived from the command).
    pub lock_key: String,
}

impl ScheduleCommand {
    /// Create a command with an explicit cron expression.
    pub fn cron(command: &'static str, expression: &str) -> Result<Self> {
        validate_expression(expression)?;
        Ok(Self {
            command,
            cron: expression.to_string(),
            frequency: format!("cron {expression}"),
            skip_if_running: false,
            on_one_server: false,
            every: Duration::ZERO,
            lock_key: format!("schedule:lock:{command}"),
        })
    }

    /// Create a command running daily at `time` (HH:MM, UTC).
    pub fn daily_at(command: &'static str, time: &str) -> Result<Self> {
        let (hour, minute) = parse_hhmm(time)?;
        let expression = format!("{minute} {hour} * * *");
        let mut cmd = Self::cron(command, &expression)?;
        cmd.frequency = format!("daily at {time}");
        Ok(cmd)
    }

    /// Create a command running every minute.
    pub fn every_minute(command: &'static str) -> Result<Self> {
        let mut cmd = Self::cron(command, "* * * * *")?;
        cmd.frequency = "every minute".to_string();
        Ok(cmd)
    }

    /// Fluent builder: skip a run when the previous one is still active.
    pub fn skip_if_running(mut self) -> Self {
        self.skip_if_running = true;
        self
    }

    /// Fluent builder: run on exactly one server via a distributed lock.
    pub fn on_one_server(mut self) -> Self {
        self.on_one_server = true;
        self
    }

    /// Compute the next UTC instant matching the cron expression at/after `from`.
    pub fn next_run(
        &self,
        from: chrono::DateTime<chrono::Utc>,
    ) -> Option<chrono::DateTime<chrono::Utc>> {
        next_cron_match(&self.cron, from)
    }
}

/// Validate a 5-field cron expression.
fn validate_expression(expression: &str) -> Result<()> {
    if expression.split_whitespace().count() != 5 {
        return Err(ScheduleError::InvalidCron(
            expression.to_string(),
            "expected 5 fields (min hour dom month dow)".into(),
        ));
    }
    for field in expression.split_whitespace() {
        if field.is_empty() {
            return Err(ScheduleError::InvalidCron(
                expression.to_string(),
                "empty field".into(),
            ));
        }
    }
    Ok(())
}

/// Parse an HH:MM wall-clock time into `(hour, minute)`.
fn parse_hhmm(time: &str) -> Result<(u32, u32)> {
    let mut parts = time.split(':');
    let (hour, minute) = match (parts.next(), parts.next(), parts.next()) {
        (Some(h), Some(m), None) => (h, m),
        _ => {
            return Err(ScheduleError::Invalid(format!(
                "daily time `{time}` must be HH:MM"
            )));
        }
    };
    let hour = hour.parse::<u32>().map_err(|_| {
        ScheduleError::Invalid(format!("daily time `{time}` has a non-numeric hour"))
    })?;
    let minute = minute.parse::<u32>().map_err(|_| {
        ScheduleError::Invalid(format!("daily time `{time}` has a non-numeric minute"))
    })?;
    if hour > 23 || minute > 59 {
        return Err(ScheduleError::Invalid(format!(
            "daily time `{time}` out of range (HH 0-23, MM 0-59)"
        )));
    }
    Ok((hour, minute))
}

/// Compute the next instant matching a 5-field cron at/after `from`.
///
/// Field semantics (all OR-expanded within a field): `*` any, `a-b` range,
/// `a,b` list, `*/n` step. Only minute/hour/day-of-month/month/day-of-week are
/// used; seconds are always zero.
fn next_cron_match(
    expression: &str,
    from: chrono::DateTime<chrono::Utc>,
) -> Option<chrono::DateTime<chrono::Utc>> {
    let fields: Vec<&str> = expression.split_whitespace().collect();
    if fields.len() != 5 {
        return None;
    }
    let minutes = parse_field(fields[0], 0, 59)?;
    let hours = parse_field(fields[1], 0, 23)?;
    let days = parse_field(fields[2], 1, 31)?;
    let months = parse_field(fields[3], 1, 12)?;
    let weekdays = parse_field(fields[4], 0, 6)?;

    // Scan minute-by-minute for up to 366 days; matches must be unambiguous.
    let start = from + chrono::Duration::minutes(1);
    let naive = start.naive_utc();
    let truncated = chrono::NaiveDate::from_ymd_opt(naive.year(), naive.month(), naive.day())
        .and_then(|d| d.and_hms_nano_opt(naive.hour(), naive.minute(), 0, 0))
        .expect("valid date components");
    let mut cursor =
        chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(truncated, chrono::Utc);
    let dom_unrestricted = is_dom_unrestricted(&days);
    for _ in 0..(366 * 24 * 60) {
        let n = cursor.naive_utc();
        let weekday = n.weekday().num_days_from_sunday();
        let dom_ok = days.contains(&n.day()) || (dom_unrestricted && weekdays.contains(&weekday));
        let month_ok = months.contains(&n.month());
        let hour_ok = hours.contains(&n.hour());
        let minute_ok = minutes.contains(&n.minute());
        if dom_ok && month_ok && hour_ok && minute_ok {
            return Some(cursor);
        }
        cursor += chrono::Duration::minutes(1);
    }
    None
}

/// Whether day-of-month is unrestricted (`*`), letting day-of-week drive.
fn is_dom_unrestricted(days: &[u32]) -> bool {
    days.len() >= 31
}

/// Expand one cron field into its allowed values.
fn parse_field(field: &str, min: u32, max: u32) -> Option<Vec<u32>> {
    let mut out = Vec::new();
    for part in field.split(',') {
        let part = part.trim();
        if part == "*" {
            for v in min..=max {
                out.push(v);
            }
        } else if let Some((base, step)) = part.split_once('/') {
            let step = step.parse::<u32>().ok()?.max(1);
            let (lo, hi) = if base == "*" {
                (min, max)
            } else if let Some((a, b)) = base.split_once('-') {
                (a.parse::<u32>().ok()?, b.parse::<u32>().ok()?)
            } else {
                let single = base.parse::<u32>().ok()?;
                (single, max)
            };
            let mut v = lo.max(min);
            while v <= hi.min(max) {
                out.push(v);
                v += step;
            }
        } else if let Some((a, b)) = part.split_once('-') {
            let a = a.parse::<u32>().ok()?;
            let b = b.parse::<u32>().ok()?;
            for v in a..=b.min(max) {
                if v >= min {
                    out.push(v);
                }
            }
        } else {
            let v = part.parse::<u32>().ok()?;
            if (min..=max).contains(&v) {
                out.push(v);
            } else {
                return None;
            }
        }
    }
    out.sort_unstable();
    out.dedup();
    Some(out)
}
