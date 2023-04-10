use serde::{Deserialize, Serialize};

pub(crate) mod badges_v1;
pub(crate) mod new;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Data<T> {
    pub(crate) data: T,
}

#[derive(Deserialize)]
pub(crate) struct Chatter {
    pub(crate) user_login: String,
}
