pub fn validate_branch_name(branch: &str) -> Result<(), String> {
    if branch.starts_with('/') || branch.ends_with('/') {
        return Err("Branch names cannot start or end with `/`.".to_string());
    }
    if branch.contains("..")
        || branch.contains("//")
        || branch.contains("@{")
        || branch.ends_with('.')
        || branch.ends_with(".lock")
    {
        return Err("Branch name contains invalid git ref syntax.".to_string());
    }
    if branch.chars().any(|character| {
        matches!(character, ' ' | '~' | '^' | ':' | '?' | '*' | '[' | '\\')
            || character.is_control()
    }) {
        return Err("Branch name contains unsupported characters.".to_string());
    }
    Ok(())
}

pub fn is_branch_char(character: char) -> bool {
    !character.is_control()
}
