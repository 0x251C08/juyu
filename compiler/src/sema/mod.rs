//! Two-pass name resolution and typechecking for the small bootstrap subset.
mod check;
mod ir;
mod types;

pub use check::analyze;
pub use ir::*;
pub use types::Type;
