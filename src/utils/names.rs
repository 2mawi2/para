use crate::utils::{ParaError, Result};
use chrono::{DateTime, Utc};
use rand::seq::SliceRandom;
use regex::Regex;

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

pub fn generate_friendly_name() -> Result<String> {
    let mut rng = rand::thread_rng();
    let adjective = ADJECTIVES
        .choose(&mut rng)
        .ok_or_else(|| ParaError::config_error("Name generation failed: adjectives list empty"))?;
    let noun = NOUNS
        .choose(&mut rng)
        .ok_or_else(|| ParaError::config_error("Name generation failed: nouns list empty"))?;
    Ok(format!("{}_{}", adjective, noun))
}

pub fn generate_timestamp() -> String {
    let now: DateTime<Utc> = Utc::now();
    now.format("%Y%m%d-%H%M%S").to_string()
}

pub fn generate_friendly_branch_name(prefix: &str, session_name: &str) -> String {
    format!("{}/{}", prefix, session_name)
}

pub fn generate_unique_name(existing_names: &[String]) -> Result<String> {
    let mut attempts = 0;
    let max_attempts = 50; // Reduced since we have 6000+ combinations

    // First, try to find a unique name without any suffix
    loop {
        let name = generate_friendly_name()?;
        if !existing_names.contains(&name) {
            return Ok(name);
        }

        attempts += 1;
        if attempts >= max_attempts {
            break;
        }
    }

    // If we can't find a unique name, try with small random suffixes
    for suffix in 1..100 {
        let name = generate_friendly_name()?;
        let candidate = format!("{}_{}", name, suffix);
        if !existing_names.contains(&candidate) {
            return Ok(candidate);
        }
    }

    // Final fallback: use timestamp suffix
    let name = generate_friendly_name()?;
    let timestamp = generate_timestamp();
    Ok(format!("{}_{}", name, timestamp))
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
        if let Some(first_char) = name.chars().next() {
            if !first_char.is_alphanumeric() {
                return Err(ParaError::invalid_session_name(
                    name,
                    "Single character session name must be alphanumeric",
                ));
            }
        } else {
            return Err(ParaError::invalid_session_name(
                name,
                "Unable to access first character of session name",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_friendly_name() {
        let name = generate_friendly_name().unwrap();
        assert!(name.contains('_'));
        assert!(name.len() > 3);

        let parts: Vec<&str> = name.split('_').collect();
        assert_eq!(parts.len(), 2);
        assert!(ADJECTIVES.contains(&parts[0]));
        assert!(NOUNS.contains(&parts[1]));
    }

    #[test]
    fn test_generate_timestamp() {
        let timestamp = generate_timestamp();
        assert_eq!(timestamp.len(), 15); // YYYYMMDD-HHMMSS
        assert!(timestamp.contains('-'));
    }

    #[test]
    fn test_generate_friendly_branch_name() {
        let branch_name = generate_friendly_branch_name("para", "epic_titanium");
        assert_eq!(branch_name, "para/epic_titanium");

        let branch_name2 = generate_friendly_branch_name("feature", "awesome_robot");
        assert_eq!(branch_name2, "feature/awesome_robot");
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
    fn test_generate_unique_name() {
        let existing = vec!["used_name".to_string(), "another_used".to_string()];
        let unique = generate_unique_name(&existing).unwrap();
        assert!(!existing.contains(&unique));
        assert!(unique.contains('_'));
    }

    #[test]
    fn test_generate_unique_name_no_collisions() {
        // Test with empty list - should generate clean name
        let existing = vec![];
        let unique = generate_unique_name(&existing).unwrap();
        assert!(unique.contains('_'));
        assert!(!unique.contains('-')); // Should be Docker-style without timestamp

        let parts: Vec<&str> = unique.split('_').collect();
        assert_eq!(parts.len(), 2); // Only adjective_noun
    }

    #[test]
    fn test_generate_unique_name_with_collision() {
        // Fill up most adjective/noun combinations to force suffix generation
        let mut existing = vec![];

        // Generate a bunch of existing names
        for i in 0..10 {
            existing.push(format!("test_name_{}", i));
        }

        // Add a specific collision to test
        existing.push("eager_alpha".to_string());

        let unique = generate_unique_name(&existing).unwrap();
        assert!(!existing.contains(&unique));
        assert!(unique.contains('_'));

        // Should either be a different adjective/noun combo or have a suffix
        if unique.starts_with("eager_alpha") {
            assert!(unique.len() > "eager_alpha".len()); // Must have suffix
        }
    }

    #[test]
    fn test_collision_avoidance_strategy() {
        // Test the three-tier collision avoidance strategy
        let mut existing = vec![];

        // First generate many unique names to test clean generation
        for _ in 0..10 {
            let name = generate_unique_name(&existing).unwrap();
            assert!(!existing.contains(&name));
            existing.push(name);
        }

        // All should be clean Docker-style names
        for name in &existing {
            let parts: Vec<&str> = name.split('_').collect();
            assert!(parts.len() <= 2 || parts.len() == 3 && parts[2].parse::<u32>().is_ok());
        }
    }
}
