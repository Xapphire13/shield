use serde::{Deserialize, Serialize};

use crate::RecordingMode;

#[derive(Serialize, Deserialize)]
pub struct SetRecordingModeInput {
    pub camera_ids: Vec<String>,
    pub mode: RecordingMode,
}
