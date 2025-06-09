// Removed unused imports - now using unified session system

pub mod archive;
pub mod recovery;
pub mod state;
pub mod manager;
pub mod migration;
pub mod validation;

pub use archive::{ArchiveEntry, ArchiveManager, ArchiveStats};
pub use recovery::{RecoveryInfo, RecoveryOptions, RecoveryResult, SessionRecovery};
pub use state::{SessionState, SessionStatus, SessionType, SessionConfig, SessionSummary, StateFileFormat};
pub use manager::{SessionManager, CreateSessionParams};
pub use migration::{StateMigrator, MigrationReport, ValidationReport as MigrationValidationReport};
pub use validation::{SessionValidator, ValidationResult, ValidationIssue, RepairResult, CleanupReport};

// Legacy types for backwards compatibility during transition
#[deprecated(note = "Use state::SessionState instead")]
pub type LegacySessionState = state::SessionState;

#[deprecated(note = "Use state::SessionStatus instead")]
pub type LegacySessionStatus = state::SessionStatus;

#[deprecated(note = "Use manager::SessionManager instead")]
pub type LegacySessionManager = manager::SessionManager;

// Tests are now in individual modules
// See: state.rs, manager.rs, migration.rs, validation.rs
