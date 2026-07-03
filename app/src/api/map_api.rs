use crate::{api::ApiClient, types::ApiError};
use shield_models::{
    Map, MapCamera, MapDoor, MapWall, UpdateMapCameraRequest, UpdateMapDoorRequest,
    UpdateMapWallRequest,
};

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

    /// `POST /maps/{map_id}/walls` — place a wall (or fence) on the map.
    async fn add_wall(&self, map_id: &str, wall: MapWall) -> Result<(), ApiError>;

    /// `PATCH /maps/{map_id}/walls/{wall_id}` — partially update a placed wall
    /// (vertices, closed, and/or color).
    async fn update_wall(
        &self,
        map_id: &str,
        wall_id: &str,
        update: UpdateMapWallRequest,
    ) -> Result<(), ApiError>;

    /// `DELETE /maps/{map_id}/walls/{wall_id}` — remove a wall from the map.
    async fn delete_wall(&self, map_id: &str, wall_id: &str) -> Result<(), ApiError>;

    /// `POST /maps/{map_id}/doors` — place a door (or gate) on the map.
    async fn add_door(&self, map_id: &str, door: MapDoor) -> Result<(), ApiError>;

    /// `PATCH /maps/{map_id}/doors/{door_id}` — partially update a placed door
    /// (start, end, and/or swing).
    async fn update_door(
        &self,
        map_id: &str,
        door_id: &str,
        update: UpdateMapDoorRequest,
    ) -> Result<(), ApiError>;

    /// `DELETE /maps/{map_id}/doors/{door_id}` — remove a door from the map.
    async fn delete_door(&self, map_id: &str, door_id: &str) -> Result<(), ApiError>;
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

    async fn add_wall(&self, map_id: &str, wall: MapWall) -> Result<(), ApiError> {
        let request = self.post(&format!("/maps/{map_id}/walls")).json(&wall);

        self.execute_with_auth(request).await?;
        Ok(())
    }

    async fn update_wall(
        &self,
        map_id: &str,
        wall_id: &str,
        update: UpdateMapWallRequest,
    ) -> Result<(), ApiError> {
        let request = self
            .patch(&format!("/maps/{map_id}/walls/{wall_id}"))
            .json(&update);

        self.execute_with_auth(request).await?;
        Ok(())
    }

    async fn delete_wall(&self, map_id: &str, wall_id: &str) -> Result<(), ApiError> {
        let request = self.delete(&format!("/maps/{map_id}/walls/{wall_id}"));

        self.execute_with_auth(request).await?;
        Ok(())
    }

    async fn add_door(&self, map_id: &str, door: MapDoor) -> Result<(), ApiError> {
        let request = self.post(&format!("/maps/{map_id}/doors")).json(&door);

        self.execute_with_auth(request).await?;
        Ok(())
    }

    async fn update_door(
        &self,
        map_id: &str,
        door_id: &str,
        update: UpdateMapDoorRequest,
    ) -> Result<(), ApiError> {
        let request = self
            .patch(&format!("/maps/{map_id}/doors/{door_id}"))
            .json(&update);

        self.execute_with_auth(request).await?;
        Ok(())
    }

    async fn delete_door(&self, map_id: &str, door_id: &str) -> Result<(), ApiError> {
        let request = self.delete(&format!("/maps/{map_id}/doors/{door_id}"));

        self.execute_with_auth(request).await?;
        Ok(())
    }
}
