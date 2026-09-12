//! Livewire variant resources — askama views enhanced with HTMX.
//!
//! The livewire kit reuses the blade askama views and overrides the base layout
//! to load HTMX; fragment partials drive targeted DOM swaps and
//! `rustasea-broadcast` pushes realtime updates (ADR-0002 decision 5).

use super::TemplateFile;

/// Livewire `resources/views` templates.
pub fn entries() -> Vec<TemplateFile> {
    // Reuse the shared askama views, replacing only the base layout.
    let mut files: Vec<TemplateFile> = super::blade::entries()
        .into_iter()
        .filter(|(path, _)| *path != "resources/views/layouts/app.html")
        .collect();
    files.push(("resources/views/layouts/app.html", LAYOUT));
    files.extend([
        ("resources/views/partials/counter.html", COUNTER),
        ("resources/views/partials/login-form.html", LOGIN_FORM),
        ("resources/views/partials/profile-form.html", PROFILE_FORM),
    ]);
    files
}

const LAYOUT: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>@@app_pascal@@</title>
  <script src="https://unpkg.com/htmx.org@2" defer></script>
</head>
<body hx-boost="true">
  <nav>
    <a href="/">@@app_pascal@@</a>
    {% if user.is_some() %}
      <a href="/dashboard">Dashboard</a>
      <a href="/settings/profile">Settings</a>
      <form method="post" action="/logout">
        <button type="submit">Log out</button>
      </form>
    {% else %}
      <a href="/login">Log in</a>
      <a href="/register">Register</a>
    {% endif %}
  </nav>

  {% include "partials/flash.html" %}

  <main id="content">
    {% block content %}{% endblock %}
  </main>
</body>
</html>
"##;

const COUNTER: &str = r##"<div id="counter" hx-target="this" hx-swap="outerHTML">
  <button hx-post="/livewire/counter/actions/increment">Increment</button>
  <span data-count>{{ count }}</span>
</div>
"##;

const LOGIN_FORM: &str = r##"<form hx-post="/login" hx-target="#content" hx-swap="innerHTML">
  <label>Email <input type="email" name="email" required /></label>
  <label>Password <input type="password" name="password" required /></label>
  <button type="submit">Log in</button>
</form>
"##;

const PROFILE_FORM: &str = r##"<form hx-patch="/settings/profile" hx-target="#content" hx-swap="innerHTML">
  <label>Name <input type="text" name="name" value="{{ user.name }}" required /></label>
  <label>Email <input type="email" name="email" value="{{ user.email }}" required /></label>
  <button type="submit">Save</button>
</form>
"##;
