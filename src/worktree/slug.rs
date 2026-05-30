const STOP_WORDS: &[&str] = &[
    "a", "an", "the", "build", "create", "add", "fix", "implement", "make", "setup", "set",
    "up", "for", "to", "and", "or",
];

/// Derive a short slug from a task title, e.g. "Build Authentication" → "auth".
pub fn task_slug(title: &str) -> String {
    let words: Vec<&str> = title
        .split_whitespace()
        .filter(|word| !STOP_WORDS.contains(&word.to_lowercase().as_str()))
        .collect();

    let word = words
        .last()
        .copied()
        .or_else(|| title.split_whitespace().last())
        .unwrap_or("tab");

    abbreviate_word(word)
}

fn abbreviate_word(word: &str) -> String {
    let lower: String = word
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect();

    if lower.is_empty() {
        return "tab".into();
    }

    if lower.len() <= 6 {
        return lower;
    }

    lower.chars().take(4).collect()
}

pub fn repo_name(repo_root: &std::path::Path) -> String {
    repo_root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repo")
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_from_task_title() {
        assert_eq!(task_slug("Build Authentication"), "auth");
    }

    #[test]
    fn slug_uses_last_meaningful_word() {
        assert_eq!(task_slug("Fix Payment Flow"), "flow");
    }

    #[test]
    fn slug_fallback_for_empty_input() {
        assert_eq!(task_slug("   "), "tab");
    }
}
