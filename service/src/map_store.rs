use std::ops::Deref;

use anyhow::Result;
use postcard::{from_bytes, to_allocvec};
use shield_models::{Map, MapCamera, UpdateMapCameraRequest};
use sled::Db;
use tracing::info;

pub struct MapStore {
    db: Db,
}

impl MapStore {
    pub fn new() -> MapStore {
        let db = sled::open("db/maps").expect("Failed to open database");

        MapStore { db }
    }

    /// Loads the stored map, or a default empty map if none exists yet.
    pub fn get_map(&self, id: &str) -> Result<Map> {
        match self.db.get(id)? {
            Some(record) => Ok(from_bytes(record.deref())?),
            None => Ok(Map {
                id: id.to_string(),
                name: "Default".into(),
                cameras: vec![],
            }),
        }
    }

    fn persist(&self, map: &Map) -> Result<()> {
        self.db.insert(&map.id, to_allocvec(map)?)?;

        Ok(())
    }

    /// Upserts a camera onto the map (replacing any existing camera with the
    /// same `camera_id`), loading or defaulting the map as needed.
    pub fn add_camera(&self, map_id: &str, camera: MapCamera) -> Result<()> {
        let mut map = self.get_map(map_id)?;

        match map
            .cameras
            .iter_mut()
            .find(|existing| existing.camera_id == camera.camera_id)
        {
            Some(existing) => *existing = camera,
            None => map.cameras.push(camera),
        }

        self.persist(&map)?;
        info!("Added camera to map {map_id}");

        Ok(())
    }

    /// Applies a partial update to a placed camera. Returns `None` if the map or
    /// the camera does not exist.
    pub fn update_camera(
        &self,
        map_id: &str,
        camera_id: &str,
        update: UpdateMapCameraRequest,
    ) -> Result<Option<MapCamera>> {
        // A missing map has no cameras, so there is nothing to update.
        if self.db.get(map_id)?.is_none() {
            return Ok(None);
        }

        let mut map = self.get_map(map_id)?;

        let Some(camera) = map
            .cameras
            .iter_mut()
            .find(|camera| camera.camera_id == camera_id)
        else {
            return Ok(None);
        };

        if let Some(position) = update.position {
            camera.position = position;
        }

        if let Some(fov) = update.fov {
            camera.fov = fov;
        }

        let updated = camera.clone();
        self.persist(&map)?;
        info!("Updated camera {camera_id} on map {map_id}");

        Ok(Some(updated))
    }

    /// Removes a placed camera. Returns `None` if the map or the camera does not
    /// exist.
    pub fn remove_camera(&self, map_id: &str, camera_id: &str) -> Result<Option<()>> {
        // A missing map has no cameras, so there is nothing to remove.
        if self.db.get(map_id)?.is_none() {
            return Ok(None);
        }

        let mut map = self.get_map(map_id)?;

        let original_len = map.cameras.len();
        map.cameras.retain(|camera| camera.camera_id != camera_id);

        if map.cameras.len() == original_len {
            return Ok(None);
        }

        self.persist(&map)?;
        info!("Removed camera {camera_id} from map {map_id}");

        Ok(Some(()))
    }
}
