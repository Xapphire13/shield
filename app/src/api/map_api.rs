use crate::{api::ApiClient, types::ApiError};
use shield_models::{Map, MapCamera, UpdateMapCameraRequest};

/// Per-element CRUD against the map resource. Mirrors [`CameraApi`](crate::api::CameraApi):
/// every call goes through [`ApiClient::execute_with_auth`] so token refresh /
/// unauthorized handling is shared.
pub trait MapApi {
    /// `GET /maps/{map_id}` — fetch a map and its placed cameras.
    async fn get_map(&self, map_id: &str) -> Result<Map, ApiError>;

    /// `POST /maps/{map_id}/cameras` — place a camera on the map.
    async fn add_camera(&self, map_id: &str, camera: MapCamera) -> Result<(), ApiError>;

    /// `PATCH /maps/{map_id}/cameras/{camera_id}` — partially update a placed
    /// camera (position and/or FOV).
    async fn update_camera(
        &self,
        map_id: &str,
        camera_id: &str,
        update: UpdateMapCameraRequest,
    ) -> Result<(), ApiError>;

    /// `DELETE /maps/{map_id}/cameras/{camera_id}` — remove a camera from the map.
    async fn delete_camera(&self, map_id: &str, camera_id: &str) -> Result<(), ApiError>;
}

impl MapApi for ApiClient {
    async fn get_map(&self, map_id: &str) -> Result<Map, ApiError> {
        let request = self.get(&format!("/maps/{map_id}"));
        Ok(self.execute_with_auth(request).await?.json().await?)
    }

    async fn add_camera(&self, map_id: &str, camera: MapCamera) -> Result<(), ApiError> {
        let request = self.post(&format!("/maps/{map_id}/cameras")).json(&camera);

        self.execute_with_auth(request).await?;
        Ok(())
    }

    async fn update_camera(
        &self,
        map_id: &str,
        camera_id: &str,
        update: UpdateMapCameraRequest,
    ) -> Result<(), ApiError> {
        let request = self
            .patch(&format!("/maps/{map_id}/cameras/{camera_id}"))
            .json(&update);

        self.execute_with_auth(request).await?;
        Ok(())
    }

    async fn delete_camera(&self, map_id: &str, camera_id: &str) -> Result<(), ApiError> {
        let request = self.delete(&format!("/maps/{map_id}/cameras/{camera_id}"));

        self.execute_with_auth(request).await?;
        Ok(())
    }
}
