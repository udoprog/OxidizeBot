//! Stream currency configuration.

/// The currency being used.
#[derive(Debug, serde::Deserialize)]
pub struct Currency {
    pub name: String,
}
