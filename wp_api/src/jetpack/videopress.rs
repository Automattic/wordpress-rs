use serde::{Deserialize, Serialize};

use crate::media::MediaDetails;

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct VideoPressMediaDetails {
    pub width: u32,
    pub height: u32,
    pub videopress: VideoPressDetails,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct VideoPressDetails {
    pub duration: u64,
    pub poster: String,
}

#[uniffi::export(with_foreign)]
pub trait VideoPressMediaDetailsExtension: Send + Sync {
    fn parse_videopress(&self, mime_type: String) -> Option<VideoPressMediaDetails>;
}

#[uniffi::export]
impl VideoPressMediaDetailsExtension for MediaDetails {
    fn parse_videopress(&self, mime_type: String) -> Option<VideoPressMediaDetails> {
        if mime_type != "video/videopress" {
            return None;
        }

        serde_json::from_str::<VideoPressMediaDetails>(self.payload.get()).ok()
    }
}
