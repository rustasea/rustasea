//! Output helpers — table / progress_bar / spinner.
//!
//! Every helper degrades gracefully in non-TTY environments (CI, captured
//! stdout): the table falls back to aligned plain text, progress bars print
//! a start/finish line, and spinners render nothing. Interactive rendering
//! uses `comfy-table` and `indicatif` behind the same functions.

use std::io::IsTerminal;

/// Renders a header + rows table.
///
/// First row is treated as the header. In a TTY a real table is drawn; in
/// CI the rows are printed as whitespace-aligned columns so assertions on
/// output stay deterministic.
pub fn table(rows: Vec<Vec<String>>) -> String {
    let mut out = String::new();
    let Some((header, body)) = rows.split_first() else {
        return out;
    };

    if std::io::stdout().is_terminal() && !header.is_empty() {
        use comfy_table::{presets::UTF8_FULL, Table};
        let mut table = Table::new();
        table.load_preset(UTF8_FULL);
        let mut header_row = comfy_table::Row::new();
        for cell in header {
            header_row.add_cell(comfy_table::Cell::new(cell));
        }
        table.set_header(header_row);
        for row in body {
            let mut table_row = comfy_table::Row::new();
            for cell in row {
                table_row.add_cell(comfy_table::Cell::new(cell));
            }
            table.add_row(table_row);
        }
        out.push_str(&table.to_string());
        return out;
    }

    // Non-TTY fallback: compute per-column widths for aligned plain text.
    let mut widths = vec![0usize; header.len()];
    for row in std::iter::once(header).chain(body.iter()) {
        for (idx, cell) in row.iter().enumerate() {
            widths[idx] = widths[idx].max(cell.chars().count());
        }
    }
    for row in std::iter::once(header).chain(body.iter()) {
        for (idx, cell) in row.iter().enumerate() {
            if idx > 0 {
                out.push_str("  ");
            }
            let padding = widths[idx].saturating_sub(cell.chars().count());
            out.push_str(cell);
            out.push_str(&" ".repeat(padding));
        }
        out.push('\n');
    }
    out
}

/// Render and print a table to stdout.
pub fn print_table(rows: Vec<Vec<String>>) {
    print!("{}", table(rows));
}

/// Progress bar helper; `total` steps over `_millis` per step.
///
/// Interactive environments render an indicatif bar; CI environments print a
/// single completion line. Returns the rendered/printed outcome as a string.
pub fn progress_bar(total: u64, _millis_per_step: u64) -> String {
    if std::io::stdout().is_terminal() {
        let bar = indicatif::ProgressBar::new(total);
        for i in 0..total {
            bar.inc(1);
            // Keep the caller loop cheap; real pacing is caller-driven.
            let _ = i;
        }
        bar.finish_and_clear();
        String::new()
    } else {
        format!("progress: {total} steps (non-TTY)\n")
    }
}

/// Spinner helper for indeterminate work.
///
/// Non-TTY environments print nothing and simply return the closure's value.
pub fn spin<T>(_label: &str, work: impl FnOnce() -> T) -> T {
    if std::io::stdout().is_terminal() {
        let spinner = indicatif::ProgressBar::new_spinner();
        spinner.enable_steady_tick(std::time::Duration::from_millis(80));
        let result = work();
        spinner.finish_and_clear();
        result
    } else {
        work()
    }
}

/// Print a one-line info message (console-styled when interactive).
pub fn info(message: impl AsRef<str>) {
    if std::io::stdout().is_terminal() {
        println!(
            "{} {}",
            console::style("INFO").cyan().bold(),
            message.as_ref()
        );
    } else {
        println!("INFO {}", message.as_ref());
    }
}

/// Print a one-line success message.
pub fn success(message: impl AsRef<str>) {
    if std::io::stdout().is_terminal() {
        println!(
            "{} {}",
            console::style("DONE").green().bold(),
            message.as_ref()
        );
    } else {
        println!("DONE {}", message.as_ref());
    }
}

/// Print a one-line warning message.
pub fn warn(message: impl AsRef<str>) {
    if std::io::stdout().is_terminal() {
        eprintln!(
            "{} {}",
            console::style("WARN").yellow().bold(),
            message.as_ref()
        );
    } else {
        eprintln!("WARN {}", message.as_ref());
    }
}

/// Whether output is a real terminal.
pub fn is_tty() -> bool {
    std::io::stdout().is_terminal()
}
