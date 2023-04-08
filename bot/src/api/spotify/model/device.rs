/// All objects related to device
use super::senum::DeviceType;
use serde::{Deserialize, Serialize};

///[get a users available devices](https://developer.spotify.com/web-api/get-a-users-available-devices/)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Device {
    pub(crate) id: String,
    pub(crate) is_active: bool,
    pub(crate) is_restricted: bool,
    pub(crate) name: String,
    #[serde(rename = "type")]
    pub(crate) _type: DeviceType,
    pub(crate) volume_percent: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct DevicePayload {
    pub(crate) devices: Vec<Device>,
}
