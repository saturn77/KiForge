pub mod types;
pub mod manager;

// Re-export the main types for easy access
pub use types::{TraceQualityType, DrcSimple, run_simple_drc_check};
pub use manager::DrcManager;