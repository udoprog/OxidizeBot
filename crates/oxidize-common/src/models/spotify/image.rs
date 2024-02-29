//! Image object

use serde::{Deserialize, Serialize};

use super::deserialize_option_u32;

///[image object](https://developer.spotify.com/web-api/object-model/#image-object)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Image {
    #[serde(deserialize_with = "deserialize_option_u32")]
    pub height: Option<u32>,
    pub url: String,
    #[serde(deserialize_with = "deserialize_option_u32")]
    pub width: Option<u32>,
}

#[test]
fn test_deserialize_float_width_height() {
    let json = r#"{"height": 640.0, "url": "https://i.scdn.co/image/ab67616d0000b273f3e3e3e3e3e3e3e3e3e3e3e3", "width": 640.0}"#;
    let image: Image = serde_json::from_str(json).unwrap();
    assert_eq!(image.height, Some(640));
    assert_eq!(image.width, Some(640));

    let json = r#"{"height": 640, "url": "https://i.scdn.co/image/ab67616d0000b273f3e3e3e3e3e3e3e3e3e3e3e3", "width": 640}"#;
    let image: Image = serde_json::from_str(json).unwrap();
    assert_eq!(image.height, Some(640));
    assert_eq!(image.width, Some(640));
}
