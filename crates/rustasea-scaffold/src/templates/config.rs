//! Generated `config/*.toml` files.
//!
//! The config loader auto-discovers every `config/*.toml` (ADR-0002 decision 8),
//! so the starter kit ships one file per concern; `inertia.toml` is emitted only
//! for the react/vue variants.

use crate::variant::StarterKitVariant;

use super::TemplateFile;

/// Config templates for `variant`.
pub fn entries(variant: StarterKitVariant) -> Vec<TemplateFile> {
    let mut files = vec![
        ("config/app.toml", APP),
        ("config/auth.toml", AUTH),
        ("config/database.toml", DATABASE),
        ("config/cache.toml", CACHE),
        ("config/queue.toml", QUEUE),
        ("config/session.toml", SESSION),
    ];
    if variant.uses_inertia() {
        files.push(("config/inertia.toml", INERTIA));
    }
    files
}

const APP: &str = r##"[app]
name = "@@app_name@@"
env = "local"
url = "http://localhost:3000"
key = ""
debug = true
"##;

const AUTH: &str = r##"[auth]
# Browser starter kits authenticate with a session cookie + CSRF; JWT stays for
# API guards.
guard = "session"
provider = "users"
password_algorithm = "argon2id"
"##;

const DATABASE: &str = r##"[database]
connection = "sqlite"
database = "database/database.sqlite"
max_connections = 5
"##;

const CACHE: &str = r##"[cache]
default = "memory"
ttl = 3600
"##;

const QUEUE: &str = r##"[queue]
default = "database"
retry_after = 90
"##;

const SESSION: &str = r##"[session]
driver = "cookie"
lifetime = 120
secure = false
http_only = true
same_site = "lax"
"##;

const INERTIA: &str = r##"[inertia]
# Asset version used for cache-busting and the 409 hard-navigation flow.
version = "1"
ssr = false
"##;
