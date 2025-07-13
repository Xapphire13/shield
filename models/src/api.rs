use serde::{Deserialize, Serialize};

use crate::RecordingMode;

#[derive(Serialize, Deserialize)]
pub struct SetRecordingModeRequest {
    pub camera_ids: Vec<String>,
    pub mode: RecordingMode,
}
