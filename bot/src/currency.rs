//! Stream currency configuration.

/// The currency being used.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Currency {
    pub name: String,
}
