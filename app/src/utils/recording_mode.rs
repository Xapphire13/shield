use shield_models::RecordingMode;

/// Extension trait for RecordingMode to provide convenient display and utility methods.
pub trait RecordingModeExtensions {
    /// Gets a human-readable display string for the recording mode
    fn display_name(&self) -> &'static str;
}

impl RecordingModeExtensions for RecordingMode {
    fn display_name(&self) -> &'static str {
        match self {
            RecordingMode::Always => "Always",
            RecordingMode::Schedule => "Schedule",
            RecordingMode::Never => "Never",
        }
    }
}
