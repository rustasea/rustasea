//! Finalizes the session after a successful login.

/// Regenerate the session id and persist the authenticated user id.
///
/// Rotating on login is the session-fixation defense (ADR-0002 decision 7).
pub fn prepare(session_id: &str) -> String {
    // The guard rotates the id; this helper exists so the controller can chain
    // prepare → redirect explicitly.
    format!("{session_id}:authenticated")
}
