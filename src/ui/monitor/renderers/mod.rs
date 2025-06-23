//! Specialized UI renderer modules for breaking down monolithic rendering logic
//!
//! This module contains focused renderers that handle specific UI concerns:
//! - `components`: Reusable UI components and builders
//! - `dialog_renderer`: All dialog types (finish, cancel, error)
//! - `table_renderer`: Session table rendering with action buttons
//! - `status_renderer`: Status bars, progress, and feedback messages
//! - `help_renderer`: Headers, footers, and help displays

pub mod components;
pub mod dialog_renderer;
pub mod help_renderer;
pub mod status_renderer;
pub mod table_renderer;

// Re-export main renderer types for easy access
pub use dialog_renderer::DialogRenderer;
pub use help_renderer::HelpRenderer;
pub use status_renderer::StatusRenderer;
pub use table_renderer::TableRenderer;

// Re-export common components for future use
#[allow(unused_imports)]
pub use components::{DialogBuilder, TableBuilder};
