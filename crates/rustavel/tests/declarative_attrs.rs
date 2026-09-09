//! Declarative attribute bundle — const emission checks (FS-M5-03, FR-506).
//!
//! Every `rustavel-macros` attribute re-emits the decorated item unchanged and
//! appends a doc-hidden helper const. Value attributes (`tries`, `backoff`,
//! `timeout`, `usage`, `help`, `queue`, `connection`) suffix the const with
//! the item's type name (`__RUSTAVEL_TRIES_<Type>`); bare markers (`hidden`,
//! `failOnTimeout`, `withoutBroadcasting`, `repairToolCalls`) emit a
//! type-less `__RUSTAVEL_<KEY>: bool`, so marker items live in their own
//! modules here to keep the const names unambiguous.
//!
//! `#[middleware]` / `#[authorize]` and `#[validate]` expansion is covered by
//! `handler_macros.rs` / `route_macro.rs` / `validate_macro.rs`.
#![allow(dead_code)] // decorated items are compile-time probes, not constructed

use rustavel::macros::{
    backoff, connection, fail_on_timeout, help, hidden, queue, repair_tool_calls, timeout, tries,
    usage, without_broadcasting,
};

// --- usize-valued attributes ---------------------------------------------

/// Job with a declarative retry budget and timing profile.
#[tries(3)]
#[backoff(10)]
#[timeout(30)]
struct SendEmailsJob;

/// Job using `0` to disable the per-attempt timeout.
#[timeout(0)]
struct LongPollJob;

#[test]
fn tries_emits_retry_budget() {
    assert_eq!(__RUSTAVEL_TRIES_SendEmailsJob, 3);
}

#[test]
fn backoff_emits_base_delay_seconds() {
    assert_eq!(__RUSTAVEL_BACKOFF_SECS_SendEmailsJob, 10);
}

#[test]
fn timeout_emits_attempt_deadline_seconds() {
    assert_eq!(__RUSTAVEL_TIMEOUT_SECS_SendEmailsJob, 30);
    assert_eq!(__RUSTAVEL_TIMEOUT_SECS_LongPollJob, 0);
}

// --- string-valued attributes --------------------------------------------

/// Console command carrying its usage/help surface.
#[usage("app:send-emails {user} [--force]")]
#[help("Dispatch the queued email campaign for a user")]
struct SendEmailsCommand;

/// Job pinned to a named queue and connection.
#[queue("emails")]
#[connection("redis")]
struct WelcomeEmailJob;

#[test]
fn usage_emits_signature_text() {
    assert_eq!(
        __RUSTAVEL_USAGE_SendEmailsCommand,
        "app:send-emails {user} [--force]"
    );
}

#[test]
fn help_emits_description_text() {
    assert_eq!(
        __RUSTAVEL_HELP_SendEmailsCommand,
        "Dispatch the queued email campaign for a user"
    );
}

#[test]
fn queue_and_connection_emit_names() {
    assert_eq!(__RUSTAVEL_QUEUE_WelcomeEmailJob, "emails");
    assert_eq!(__RUSTAVEL_CONNECTION_WelcomeEmailJob, "redis");
}

// --- bare marker attributes (type-less consts, one item per module) -------

mod hidden_marker {
    use super::hidden;

    /// Command omitted from `list` unless `--all` is passed.
    #[hidden]
    struct SecretMaintenanceCommand;

    /// Type-less marker const is private to the module it decorates.
    #[test]
    fn emits_marker() {
        assert!(__RUSTAVEL_HIDDEN);
    }
}

mod fail_on_timeout_marker {
    use super::fail_on_timeout;

    /// Job that fails instead of releasing a timed-out attempt.
    #[fail_on_timeout]
    struct DeadlineStrictJob;

    /// Type-less marker const is private to the module it decorates.
    #[test]
    fn emits_marker() {
        assert!(__RUSTAVEL_FAIL_ON_TIMEOUT);
    }
}

mod without_broadcasting_marker {
    use super::without_broadcasting;

    /// Event whose fan-out is suppressed by the dispatcher.
    #[without_broadcasting]
    struct UserEmailChanged;

    /// Type-less marker const is private to the module it decorates.
    #[test]
    fn emits_marker() {
        assert!(__RUSTAVEL_WITHOUT_BROADCASTING);
    }
}

mod repair_tool_calls_marker {
    use super::repair_tool_calls;

    /// Agent/tool that enables AI tool-call repair (M6).
    #[repair_tool_calls]
    struct SearchAssistant;

    /// Type-less marker const is private to the module it decorates.
    #[test]
    fn emits_marker() {
        assert!(__RUSTAVEL_REPAIR_TOOL_CALLS);
    }
}

// --- combination + item preservation -------------------------------------

/// Job carrying value attrs and markers together; the struct body must
/// survive untouched.
#[queue("high")]
#[tries(5)]
struct PriorityIngestJob;

/// Hidden marker for the combined item — own module keeps the type-less
/// `__RUSTAVEL_HIDDEN` const unique.
mod priority_marker {
    use super::hidden;

    /// Own marker keeps the type-less const unique next to the suffixed ones.
    #[hidden]
    struct _CollisionFree;

    #[test]
    fn item_survives_and_consts_coexist() {
        assert_eq!(super::__RUSTAVEL_TRIES_PriorityIngestJob, 5);
        assert_eq!(super::__RUSTAVEL_QUEUE_PriorityIngestJob, "high");
        assert!(__RUSTAVEL_HIDDEN);
        // Struct body untouched: name + field still resolvable.
        let _ = core::mem::size_of::<super::PriorityIngestJob>();
    }
}
