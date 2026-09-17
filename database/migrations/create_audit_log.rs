//! Creates the `audit_log` table (activity/audit trail).

use rustasea::orm::Migration;
use rustasea::OrmResult;

/// `create_audit_log_table` migration.
pub struct CreateAuditLog;

impl Migration for CreateAuditLog {
    fn name(&self) -> &str {
        "2027_01_01_000008_create_audit_log_table"
    }

    fn up(&self) -> OrmResult<String> {
        Ok(r#"CREATE TABLE audit_log (
    id UUID PRIMARY KEY,
    batch_uuid UUID NULL,
    log_name VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    subject_type VARCHAR(255) NULL,
    subject_id UUID NULL,
    causer_type VARCHAR(255) NULL,
    causer_id VARCHAR(255) NULL,
    properties JSONB NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX audit_log_subject_index ON audit_log (subject_type, subject_id);
CREATE INDEX audit_log_causer_index ON audit_log (causer_type, causer_id);
CREATE INDEX audit_log_batch_uuid_index ON audit_log (batch_uuid);
CREATE INDEX audit_log_log_name_index ON audit_log (log_name);"#
        .to_string())
    }

    fn down(&self) -> OrmResult<String> {
        Ok("DROP TABLE IF EXISTS audit_log;".to_string())
    }
}
