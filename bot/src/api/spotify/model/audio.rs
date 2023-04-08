//! All objects related to artist defined by Spotify API

use serde::{Deserialize, Serialize};
///[audio feature object](https://developer.spotify.com/web-api/object-model/#audio-features-object)
/// Audio Feature object
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct AudioFeatures {
    pub(crate) acousticness: f32,
    pub(crate) analysis_url: String,
    pub(crate) danceability: f32,
    pub(crate) duration_ms: u32,
    pub(crate) energy: f32,
    pub(crate) id: String,
    pub(crate) instrumentalness: f32,
    pub(crate) key: i32,
    pub(crate) liveness: f32,
    pub(crate) loudness: f32,
    pub(crate) mode: f32,
    pub(crate) speechiness: f32,
    pub(crate) tempo: f32,
    pub(crate) time_signature: i32,
    pub(crate) track_href: String,
    #[serde(rename = "type")]
    pub(crate) _type: String,
    pub(crate) uri: String,
    pub(crate) valence: f32,
}

/// Audio Feature Vector
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct AudioFeaturesPayload {
    pub(crate) audio_features: Vec<AudioFeatures>,
}

/// Audio Analysis Object
///[audio analysis](https://developer.spotify.com/web-api/get-audio-analysis/)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct AudioAnalysis {
    pub(crate) bars: Vec<AudioAnalysisMeasure>,
    pub(crate) beats: Vec<AudioAnalysisMeasure>,
    pub(crate) meta: AudioAnalysisMeta,
    pub(crate) sections: Vec<AudioAnalysisSection>,
    pub(crate) segments: Vec<AudioAnalysisSegment>,
    pub(crate) tatums: Vec<AudioAnalysisMeasure>,
    pub(crate) track: AudioAnalysisTrack,
}

///[audio analysis](https://developer.spotify.com/web-api/get-audio-analysis/)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct AudioAnalysisMeasure {
    pub(crate) start: f32,
    pub(crate) duration: f32,
    pub(crate) confidence: f32,
}

///[audio analysis](https://developer.spotify.com/web-api/get-audio-analysis/)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct AudioAnalysisSection {
    pub(crate) start: f32,
    pub(crate) duration: f32,
    pub(crate) confidence: f32,
    pub(crate) loudness: f32,
    pub(crate) tempo: f32,
    pub(crate) tempo_confidence: f32,
    pub(crate) key: i32,
    pub(crate) key_confidence: f32,
    pub(crate) mode: f32,
    pub(crate) mode_confidence: f32,
    pub(crate) time_signature: i32,
    pub(crate) time_signature_confidence: f32,
}

///[audio analysis](https://developer.spotify.com/web-api/get-audio-analysis/)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct AudioAnalysisMeta {
    pub(crate) analyzer_version: String,
    pub(crate) platform: String,
    pub(crate) detailed_status: String,
    pub(crate) status_code: i32,
    pub(crate) timestamp: u64,
    pub(crate) analysis_time: f32,
    pub(crate) input_process: String,
}
///[audio analysis](https://developer.spotify.com/web-api/get-audio-analysis/)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct AudioAnalysisSegment {
    pub(crate) start: f32,
    pub(crate) duration: f32,
    pub(crate) confidence: f32,
    pub(crate) loudness_start: f32,
    pub(crate) loudness_max_time: f32,
    pub(crate) loudness_max: f32,
    pub(crate) loudness_end: Option<f32>,
    pub(crate) pitches: Vec<f32>,
    pub(crate) timbre: Vec<f32>,
}

///[audio analysis](https://developer.spotify.com/web-api/get-audio-analysis/)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct AudioAnalysisTrack {
    pub(crate) num_samples: u32,
    pub(crate) duration: f32,
    pub(crate) sample_md5: String,
    pub(crate) offset_seconds: u32,
    pub(crate) window_seconds: u32,
    pub(crate) analysis_sample_rate: i32,
    pub(crate) analysis_channels: u32,
    pub(crate) end_of_fade_in: f32,
    pub(crate) start_of_fade_out: f32,
    pub(crate) loudness: f32,
    pub(crate) tempo: f32,
    pub(crate) tempo_confidence: f32,
    pub(crate) time_signature: i32,
    pub(crate) time_signature_confidence: f32,
    pub(crate) key: u32,
    pub(crate) key_confidence: f32,
    pub(crate) mode: f32,
    pub(crate) mode_confidence: f32,
    pub(crate) codestring: String,
    pub(crate) code_version: f32,
    pub(crate) echoprintstring: String,
    pub(crate) echoprint_version: f32,
    pub(crate) synchstring: String,
    pub(crate) synch_version: f32,
    pub(crate) rhythmstring: String,
    pub(crate) rhythm_version: f32,
}
