use crate::zoxide;

pub(crate) fn validate_session_name(session_name: &str) -> Result<(), String> {
    if session_name.is_empty() {
        return Err("Session name cannot be empty.".to_string());
    }
    if session_name.contains('/') {
        return Err("Session name cannot contain '/'.".to_string());
    }
    if session_name.len() > zoxide::MAX_SESSION_NAME_LEN {
        return Err(format!(
            "Session name must be at most {} bytes.",
            zoxide::MAX_SESSION_NAME_LEN
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_session_names() {
        assert!(validate_session_name("dev").is_ok());
        assert!(validate_session_name("").is_err());
        assert!(validate_session_name("dev/test").is_err());
        assert!(validate_session_name(&"a".repeat(zoxide::MAX_SESSION_NAME_LEN)).is_ok());
        assert!(validate_session_name(&"a".repeat(zoxide::MAX_SESSION_NAME_LEN + 1)).is_err());
    }
}
