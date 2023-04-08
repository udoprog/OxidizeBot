use serde::{Deserialize, Serialize};

pub mod badges_v1;
pub mod new;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Data<T> {
    pub data: T,
}

#[derive(Deserialize)]
pub struct Chatter {
    pub user_id: String,
    pub user_login: String,
    pub user_name: String,
}

/// Response from the validate token endpoint.
#[derive(Debug, Deserialize)]
pub struct ValidateToken {
    pub client_id: String,
    pub login: String,
    pub scopes: Vec<String>,
    pub user_id: String,
}
