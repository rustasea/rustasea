//! Profile validation rules shared by registration and settings.

/// Maximum accepted display-name length.
pub const NAME_MAX: usize = 255;

/// Validate a display name.
pub fn validate_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("name is required".to_string());
    }
    if name.chars().count() > NAME_MAX {
        return Err(format!("name may not exceed {NAME_MAX} characters"));
    }
    Ok(())
}
