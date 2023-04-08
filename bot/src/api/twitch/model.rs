use serde::{Deserialize, Serialize};

pub(crate) mod badges_v1;
pub(crate) mod new;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Data<T> {
    pub(crate) data: T,
}

#[derive(Deserialize)]
pub(crate) struct Chatter {
    pub(crate) user_id: String,
    pub(crate) user_login: String,
    pub(crate) user_name: String,
}

/// Response from the validate token endpoint.
#[derive(Debug, Deserialize)]
pub(crate) struct ValidateToken {
    pub(crate) client_id: String,
    pub(crate) login: String,
    pub(crate) scopes: Vec<String>,
    pub(crate) user_id: String,
}
