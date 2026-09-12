//! Blade variant resources — server-rendered askama views.
//!
//! Templates live under `resources/views` and are compiled into the binary by
//! askama (`askama.toml` points the engine at that directory).

use super::TemplateFile;

/// Blade `resources/views` templates.
pub fn entries() -> Vec<TemplateFile> {
    vec![
        ("resources/views/layouts/app.html", LAYOUT),
        ("resources/views/dashboard.html", DASHBOARD),
        ("resources/views/auth/login.html", LOGIN),
        ("resources/views/auth/register.html", REGISTER),
        ("resources/views/auth/forgot-password.html", FORGOT_PASSWORD),
        ("resources/views/auth/reset-password.html", RESET_PASSWORD),
        ("resources/views/settings/profile.html", SETTINGS_PROFILE),
        ("resources/views/settings/password.html", SETTINGS_PASSWORD),
        ("resources/views/partials/flash.html", FLASH),
    ]
}

const LAYOUT: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>@@app_pascal@@</title>
</head>
<body>
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

  <main>
    {% block content %}{% endblock %}
  </main>
</body>
</html>
"##;

const DASHBOARD: &str = r##"{% extends "layouts/app.html" %}
{% block content %}
<h1>Dashboard</h1>
<p>Welcome back, {{ user.name }}.</p>
{% endblock %}
"##;

const LOGIN: &str = r##"{% extends "layouts/app.html" %}
{% block content %}
<h1>Log in</h1>
<form method="post" action="/login">
  <label>Email <input type="email" name="email" required /></label>
  <label>Password <input type="password" name="password" required /></label>
  <button type="submit">Log in</button>
</form>
<p><a href="/forgot-password">Forgot your password?</a></p>
{% endblock %}
"##;

const REGISTER: &str = r##"{% extends "layouts/app.html" %}
{% block content %}
<h1>Register</h1>
<form method="post" action="/register">
  <label>Name <input type="text" name="name" required /></label>
  <label>Email <input type="email" name="email" required /></label>
  <label>Password <input type="password" name="password" required /></label>
  <label>Confirm <input type="password" name="password_confirmation" required /></label>
  <button type="submit">Create account</button>
</form>
{% endblock %}
"##;

const FORGOT_PASSWORD: &str = r##"{% extends "layouts/app.html" %}
{% block content %}
<h1>Forgot password</h1>
<form method="post" action="/forgot-password">
  <label>Email <input type="email" name="email" required /></label>
  <button type="submit">Email reset link</button>
</form>
{% endblock %}
"##;

const RESET_PASSWORD: &str = r##"{% extends "layouts/app.html" %}
{% block content %}
<h1>Reset password</h1>
<form method="post" action="/reset-password">
  <input type="hidden" name="token" value="{{ token }}" />
  <label>Email <input type="email" name="email" required /></label>
  <label>New password <input type="password" name="password" required /></label>
  <label>Confirm <input type="password" name="password_confirmation" required /></label>
  <button type="submit">Reset password</button>
</form>
{% endblock %}
"##;

const SETTINGS_PROFILE: &str = r##"{% extends "layouts/app.html" %}
{% block content %}
<h1>Profile</h1>
<form method="post" action="/settings/profile">
  <input type="hidden" name="_method" value="PATCH" />
  <label>Name <input type="text" name="name" value="{{ user.name }}" required /></label>
  <label>Email <input type="email" name="email" value="{{ user.email }}" required /></label>
  <button type="submit">Save</button>
</form>
{% endblock %}
"##;

const SETTINGS_PASSWORD: &str = r##"{% extends "layouts/app.html" %}
{% block content %}
<h1>Password</h1>
<form method="post" action="/settings/password">
  <input type="hidden" name="_method" value="PUT" />
  <label>Current password <input type="password" name="current_password" required /></label>
  <label>New password <input type="password" name="password" required /></label>
  <label>Confirm <input type="password" name="password_confirmation" required /></label>
  <button type="submit">Update password</button>
</form>
{% endblock %}
"##;

const FLASH: &str = r##"{% if flash.is_some() %}
<div role="status">{{ flash }}</div>
{% endif %}
"##;
