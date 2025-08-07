use crate::{api::ApiClient, types::ApiError};
use shield_models::{Camera, RecordingMode, SetRecordingModeRequest};

pub trait CameraApi {
    async fn get_cameras(&self) -> Result<Vec<Camera>, ApiError>;
    async fn set_recording_mode(&self, request: SetRecordingModeRequest) -> Result<(), ApiError>;

    // Convenience methods
    async fn update_recording_mode(
        &self,
        camera_ids: Vec<String>,
        mode: RecordingMode,
    ) -> Result<(), ApiError> {
        let request = SetRecordingModeRequest { camera_ids, mode };
        self.set_recording_mode(request).await
    }
}

impl CameraApi for ApiClient {
    async fn get_cameras(&self) -> Result<Vec<shield_models::Camera>, ApiError> {
        let request = self.get("/cameras");
        Ok(self.execute_with_auth(request).await?.json().await?)
    }

    async fn set_recording_mode(&self, request: SetRecordingModeRequest) -> Result<(), ApiError> {
        let req = self.post("/set_recording_mode").json(&request);

        self.execute_with_auth(req).await?;
        Ok(())
    }
}
