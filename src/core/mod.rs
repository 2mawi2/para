pub mod claude_launcher;
pub mod daemon;
pub mod docker;
pub mod git;
pub mod ide;
pub mod session;
pub mod status;

// Docker module will be conditionally compiled once feature is added to Cargo.toml
// #[cfg(feature = "docker")]
// pub mod docker;
