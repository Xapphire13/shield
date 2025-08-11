use serde::{Deserialize, Serialize};

use crate::RecordingMode;

#[derive(Serialize, Deserialize, Clone)]
pub struct SetRecordingModeRequest {
    pub camera_ids: Vec<String>,
    pub mode: RecordingMode,
}
