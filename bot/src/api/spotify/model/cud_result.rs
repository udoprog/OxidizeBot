//! the result of post/put/delete request
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct CUDResult {
    pub(crate) snapshot_id: String,
}
