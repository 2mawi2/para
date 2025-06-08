use crate::utils::{ParaError, Result};
use chrono::{DateTime, Utc};
use rand::seq::SliceRandom;
use rand::Rng;
use regex::Regex;
use std::fmt;

const ADJECTIVES: &[&str] = &[
    "agile",
    "bold",
    "calm",
    "deep",
    "eager",
    "fast",
    "keen",
    "neat",
    "quick",
    "smart",
    "swift",
    "wise",
    "zesty",
    "bright",
    "clever",
    "dynamic",
    "elegant",
    "fresh",
    "gentle",
    "happy",
    "intense",
    "jovial",
    "lively",
    "modern",
    "nimble",
    "optimistic",
    "polished",
    "quiet",
    "robust",
    "sleek",
    "tender",
    "unique",
    "vibrant",
    "warm",
    "xenial",
    "youthful",
    "zealous",
    "active",
    "brave",
    "crisp",
    "daring",
    "epic",
    "fluid",
    "golden",
    "heroic",
    "ideal",
    "jazzy",
    "kinetic",
    "lucid",
    "magical",
    "noble",
    "organic",
    "perfect",
    "radiant",
    "serene",
    "timeless",
    "unstoppable",
    "vivid",
    "wonderful",
    "excellent",
    "young",
];

const NOUNS: &[&str] = &[
    "alpha", "beta", "gamma", "delta", "omega", "sigma", "theta", "lambda", "aurora", "cosmos",
    "nebula", "quasar", "pulsar", "galaxy", "comet", "meteor", "planet", "stellar", "lunar",
    "solar", "crystal", "diamond", "emerald", "sapphire", "ruby", "amber", "pearl", "coral",
    "jade", "opal", "topaz", "obsidian", "granite", "marble", "bronze", "silver", "platinum",
    "titanium", "cobalt", "copper", "iron", "steel", "carbon", "helium", "neon", "argon", "xenon",
    "radon", "krypton", "mercury", "phoenix", "dragon", "falcon", "eagle", "hawk", "raven", "dove",
    "swan", "crane", "heron", "owl", "robin", "sparrow", "wren", "oak", "pine", "maple", "birch",
    "cedar", "willow", "elm", "ash", "palm", "bamboo", "fern", "moss", "ivy", "vine", "rose",
    "lily", "iris", "tulip", "daisy", "orchid", "lotus", "jasmine", "lavender", "mint", "sage",
    "basil", "thyme", "rosemary", "ginger", "cinnamon", "vanilla", "honey", "sugar", "spice",
    "pepper", "salt", "lemon", "lime", "orange", "apple", "cherry", "berry", "grape", "peach",
];

pub fn generate_friendly_name() -> String {
    let mut rng = rand::thread_rng();
    let adjective = ADJECTIVES.choose(&mut rng).unwrap();
    let noun = NOUNS.choose(&mut rng).unwrap();
    format!("{}_{}", adjective, noun)
}

pub fn generate_session_id() -> String {
    let timestamp = generate_timestamp();
    let friendly = generate_friendly_name();
    format!("{}_{}", friendly, timestamp)
}

pub fn generate_timestamp() -> String {
    let now: DateTime<Utc> = Utc::now();
    now.format("%Y%m%d-%H%M%S").to_string()
}

pub fn generate_branch_name(prefix: &str) -> String {
    let timestamp = generate_timestamp();
    format!("{}/{}", prefix, timestamp)
}

pub fn generate_unique_name(existing_names: &[String]) -> String {
    let mut attempts = 0;
    let max_attempts = 100;

    loop {
        let name = generate_friendly_name();
        if !existing_names.contains(&name) {
            return name;
        }

        attempts += 1;
        if attempts >= max_attempts {
            let mut rng = rand::thread_rng();
            let suffix: u32 = rng.gen_range(1000..9999);
            return format!("{}_{}", name, suffix);
        }
    }
}

pub fn validate_session_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(ParaError::invalid_session_name(
            name,
            "Session name cannot be empty",
        ));
    }

    if name.len() > 100 {
        return Err(ParaError::invalid_session_name(
            name,
            "Session name cannot be longer than 100 characters",
        ));
    }

    let valid_regex = Regex::new(r"^[a-zA-Z0-9][a-zA-Z0-9_-]*[a-zA-Z0-9]$")
        .map_err(|e| ParaError::config_error(format!("Invalid regex: {}", e)))?;

    if name.len() == 1 {
        if !name.chars().next().unwrap().is_alphanumeric() {
            return Err(ParaError::invalid_session_name(
                name,
                "Single character session name must be alphanumeric",
            ));
        }
    } else if !valid_regex.is_match(name) {
        return Err(ParaError::invalid_session_name(
            name,
            "Session name must start and end with alphanumeric characters and contain only letters, numbers, hyphens, and underscores"
        ));
    }

    if name.starts_with('-') || name.ends_with('-') {
        return Err(ParaError::invalid_session_name(
            name,
            "Session name cannot start or end with a hyphen",
        ));
    }

    if name.starts_with('_') || name.ends_with('_') {
        return Err(ParaError::invalid_session_name(
            name,
            "Session name cannot start or end with an underscore",
        ));
    }

    if name.contains("__") || name.contains("--") {
        return Err(ParaError::invalid_session_name(
            name,
            "Session name cannot contain consecutive underscores or hyphens",
        ));
    }

    Ok(())
}

pub fn sanitize_branch_name(name: &str) -> String {
    let mut result = name.to_string();

    result = result.replace(' ', "-");
    result = result.replace('\t', "-");
    result = result.replace('\n', "-");
    result = result.replace('\r', "");

    let invalid_chars = ['~', '^', ':', '?', '*', '[', ']', '\\', '/', '@', '{', '}'];
    for ch in invalid_chars {
        result = result.replace(ch, "");
    }

    result = result.replace("..", "");

    while result.contains("--") {
        result = result.replace("--", "-");
    }

    result = result.trim_matches('-').to_string();
    result = result.trim_matches('.').to_string();

    if result.is_empty() {
        result = "branch".to_string();
    }

    result
}

pub fn validate_branch_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(ParaError::invalid_branch_name(
            name,
            "Branch name cannot be empty",
        ));
    }

    if name.len() > 250 {
        return Err(ParaError::invalid_branch_name(
            name,
            "Branch name cannot be longer than 250 characters",
        ));
    }

    if name.starts_with('-') || name.ends_with('-') {
        return Err(ParaError::invalid_branch_name(
            name,
            "Branch name cannot start or end with a hyphen",
        ));
    }

    if name.starts_with('.') || name.ends_with('.') {
        return Err(ParaError::invalid_branch_name(
            name,
            "Branch name cannot start or end with a dot",
        ));
    }

    if name.contains("..") {
        return Err(ParaError::invalid_branch_name(
            name,
            "Branch name cannot contain consecutive dots",
        ));
    }

    if name.contains("//") {
        return Err(ParaError::invalid_branch_name(
            name,
            "Branch name cannot contain consecutive slashes",
        ));
    }

    let invalid_chars = [
        '~', '^', ':', '?', '*', '[', ']', '\\', ' ', '\t', '\n', '\r', '@', '{', '}',
    ];
    for ch in invalid_chars {
        if name.contains(ch) {
            return Err(ParaError::invalid_branch_name(
                name,
                format!("Branch name cannot contain character: {}", ch),
            ));
        }
    }

    if name == "@" {
        return Err(ParaError::invalid_branch_name(
            name,
            "Branch name cannot be '@'",
        ));
    }

    Ok(())
}

pub fn extract_session_name_from_branch(branch_name: &str) -> Option<String> {
    if let Some(parts) = branch_name.strip_prefix("para/") {
        Some(parts.to_string())
    } else if let Some(parts) = branch_name.strip_prefix("pc/") {
        Some(parts.to_string())
    } else {
        None
    }
}

pub fn is_para_branch(branch_name: &str) -> bool {
    branch_name.starts_with("para/") || branch_name.starts_with("pc/")
}

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub name: String,
    pub branch: String,
    pub timestamp: String,
    pub friendly_name: Option<String>,
}

impl SessionInfo {
    pub fn new(name: String) -> Self {
        let timestamp = generate_timestamp();
        let branch = format!("para/{}", name);

        Self {
            name,
            branch,
            timestamp,
            friendly_name: None,
        }
    }

    pub fn from_branch(branch_name: &str) -> Option<Self> {
        if let Some(session_name) = extract_session_name_from_branch(branch_name) {
            Some(Self {
                name: session_name.clone(),
                branch: branch_name.to_string(),
                timestamp: extract_timestamp_from_name(&session_name)
                    .unwrap_or_else(|| "unknown".to_string()),
                friendly_name: extract_friendly_name(&session_name),
            })
        } else {
            None
        }
    }
}

impl fmt::Display for SessionInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(friendly) = &self.friendly_name {
            write!(f, "{} ({})", friendly, self.timestamp)
        } else {
            write!(f, "{}", self.name)
        }
    }
}

fn extract_timestamp_from_name(name: &str) -> Option<String> {
    let timestamp_regex = Regex::new(r"(\d{8}-\d{6})").ok()?;
    timestamp_regex.find(name).map(|m| m.as_str().to_string())
}

fn extract_friendly_name(name: &str) -> Option<String> {
    if let Some(pos) = name.rfind('_') {
        let potential_friendly = &name[..pos];
        if potential_friendly.contains('_') && potential_friendly.len() > 3 {
            return Some(potential_friendly.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_friendly_name() {
        let name = generate_friendly_name();
        assert!(name.contains('_'));
        assert!(name.len() > 3);

        let parts: Vec<&str> = name.split('_').collect();
        assert_eq!(parts.len(), 2);
        assert!(ADJECTIVES.contains(&parts[0]));
        assert!(NOUNS.contains(&parts[1]));
    }

    #[test]
    fn test_generate_session_id() {
        let id = generate_session_id();
        assert!(id.contains('_'));
        assert!(id.len() > 10);
    }

    #[test]
    fn test_generate_timestamp() {
        let timestamp = generate_timestamp();
        assert_eq!(timestamp.len(), 15); // YYYYMMDD-HHMMSS
        assert!(timestamp.contains('-'));
    }

    #[test]
    fn test_validate_session_name() {
        assert!(validate_session_name("valid-name").is_ok());
        assert!(validate_session_name("valid_name").is_ok());
        assert!(validate_session_name("valid123").is_ok());
        assert!(validate_session_name("a").is_ok());
        assert!(validate_session_name("123").is_ok());

        assert!(validate_session_name("").is_err());
        assert!(validate_session_name("-invalid").is_err());
        assert!(validate_session_name("invalid-").is_err());
        assert!(validate_session_name("_invalid").is_err());
        assert!(validate_session_name("invalid_").is_err());
        assert!(validate_session_name("invalid--name").is_err());
        assert!(validate_session_name("invalid__name").is_err());
        assert!(validate_session_name("invalid name").is_err());
        assert!(validate_session_name("invalid@name").is_err());

        let long_name = "a".repeat(101);
        assert!(validate_session_name(&long_name).is_err());
    }

    #[test]
    fn test_sanitize_branch_name() {
        assert_eq!(sanitize_branch_name("valid name"), "valid-name");
        assert_eq!(sanitize_branch_name("with\ttabs"), "with-tabs");
        assert_eq!(sanitize_branch_name("with/slashes"), "withslashes");
        assert_eq!(sanitize_branch_name("with..dots"), "withdots");
        assert_eq!(
            sanitize_branch_name("--multiple--dashes--"),
            "multiple-dashes"
        );
        assert_eq!(sanitize_branch_name(""), "branch");
        assert_eq!(sanitize_branch_name("---"), "branch");
    }

    #[test]
    fn test_validate_branch_name() {
        assert!(validate_branch_name("valid-branch").is_ok());
        assert!(validate_branch_name("feature/new-feature").is_ok());
        assert!(validate_branch_name("123").is_ok());

        assert!(validate_branch_name("").is_err());
        assert!(validate_branch_name("-invalid").is_err());
        assert!(validate_branch_name("invalid-").is_err());
        assert!(validate_branch_name(".invalid").is_err());
        assert!(validate_branch_name("invalid.").is_err());
        assert!(validate_branch_name("invalid..name").is_err());
        assert!(validate_branch_name("invalid//name").is_err());
        assert!(validate_branch_name("invalid name").is_err());
        assert!(validate_branch_name("invalid@name").is_err());
        assert!(validate_branch_name("@").is_err());

        let long_name = "a".repeat(251);
        assert!(validate_branch_name(&long_name).is_err());
    }

    #[test]
    fn test_extract_session_name_from_branch() {
        assert_eq!(
            extract_session_name_from_branch("para/my-session"),
            Some("my-session".to_string())
        );
        assert_eq!(
            extract_session_name_from_branch("pc/my-session"),
            Some("my-session".to_string())
        );
        assert_eq!(extract_session_name_from_branch("feature/my-feature"), None);
    }

    #[test]
    fn test_is_para_branch() {
        assert!(is_para_branch("para/my-session"));
        assert!(is_para_branch("pc/my-session"));
        assert!(!is_para_branch("feature/my-feature"));
        assert!(!is_para_branch("main"));
    }

    #[test]
    fn test_session_info() {
        let session = SessionInfo::new("test-session".to_string());
        assert_eq!(session.name, "test-session");
        assert!(session.branch.starts_with("para/"));
        assert!(!session.timestamp.is_empty());

        let from_branch = SessionInfo::from_branch("para/test-session").unwrap();
        assert_eq!(from_branch.name, "test-session");
        assert_eq!(from_branch.branch, "para/test-session");
    }

    #[test]
    fn test_generate_unique_name() {
        let existing = vec!["used_name".to_string(), "another_used".to_string()];
        let unique = generate_unique_name(&existing);
        assert!(!existing.contains(&unique));
        assert!(unique.contains('_'));
    }
}
