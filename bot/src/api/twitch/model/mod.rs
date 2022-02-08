use serde::{Deserialize, Serialize};

pub mod badges_v1;
pub mod new;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Data<T> {
    pub data: T,
}

#[derive(Deserialize)]
pub struct Chatters {
    #[serde(default)]
    pub broadcaster: Vec<String>,
    #[serde(default)]
    pub vips: Vec<String>,
    #[serde(default)]
    pub moderators: Vec<String>,
    #[serde(default)]
    pub staff: Vec<String>,
    #[serde(default)]
    pub admins: Vec<String>,
    #[serde(default)]
    pub global_mods: Vec<String>,
    #[serde(default)]
    pub viewers: Vec<String>,
}

/// Response from the validate token endpoint.
#[derive(Debug, Deserialize)]
pub struct ValidateToken {
    pub client_id: String,
    pub login: String,
    pub scopes: Vec<String>,
    pub user_id: String,
}
