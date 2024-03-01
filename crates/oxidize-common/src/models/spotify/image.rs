//! Image object

use serde::{Deserialize, Serialize};

///[image object](https://developer.spotify.com/web-api/object-model/#image-object)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Image {
    #[serde(default, deserialize_with = "super::deserialize_option_number")]
    pub height: Option<u32>,
    pub url: String,
    #[serde(default, deserialize_with = "super::deserialize_option_number")]
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
