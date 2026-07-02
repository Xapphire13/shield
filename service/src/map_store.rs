use std::ops::Deref;

use anyhow::Result;
use postcard::{from_bytes, to_allocvec};
use shield_models::{
    Map, MapCamera, MapDoor, MapWall, UpdateMapCameraRequest, UpdateMapDoorRequest,
    UpdateMapWallRequest,
};
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
                walls: vec![],
                doors: vec![],
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

    /// Upserts a wall onto the map (replacing any existing wall with the
    /// same `id`), loading or defaulting the map as needed.
    pub fn add_wall(&self, map_id: &str, wall: MapWall) -> Result<()> {
        let mut map = self.get_map(map_id)?;

        match map.walls.iter_mut().find(|existing| existing.id == wall.id) {
            Some(existing) => *existing = wall,
            None => map.walls.push(wall),
        }

        self.persist(&map)?;
        info!("Added wall to map {map_id}");

        Ok(())
    }

    /// Applies a partial update to a placed wall. Returns `None` if the map or
    /// the wall does not exist.
    pub fn update_wall(
        &self,
        map_id: &str,
        wall_id: &str,
        update: UpdateMapWallRequest,
    ) -> Result<Option<MapWall>> {
        // A missing map has no walls, so there is nothing to update.
        if self.db.get(map_id)?.is_none() {
            return Ok(None);
        }

        let mut map = self.get_map(map_id)?;

        let Some(wall) = map.walls.iter_mut().find(|wall| wall.id == wall_id) else {
            return Ok(None);
        };

        if let Some(vertices) = update.vertices {
            wall.vertices = vertices;
        }

        if let Some(closed) = update.closed {
            wall.closed = closed;
        }

        if let Some(color) = update.color {
            wall.color = color;
        }

        let updated = wall.clone();
        self.persist(&map)?;
        info!("Updated wall {wall_id} on map {map_id}");

        Ok(Some(updated))
    }

    /// Removes a placed wall. Returns `None` if the map or the wall does not
    /// exist.
    pub fn remove_wall(&self, map_id: &str, wall_id: &str) -> Result<Option<()>> {
        // A missing map has no walls, so there is nothing to remove.
        if self.db.get(map_id)?.is_none() {
            return Ok(None);
        }

        let mut map = self.get_map(map_id)?;

        let original_len = map.walls.len();
        map.walls.retain(|wall| wall.id != wall_id);

        if map.walls.len() == original_len {
            return Ok(None);
        }

        self.persist(&map)?;
        info!("Removed wall {wall_id} from map {map_id}");

        Ok(Some(()))
    }

    /// Upserts a door onto the map (replacing any existing door with the
    /// same `id`), loading or defaulting the map as needed.
    pub fn add_door(&self, map_id: &str, door: MapDoor) -> Result<()> {
        let mut map = self.get_map(map_id)?;

        match map.doors.iter_mut().find(|existing| existing.id == door.id) {
            Some(existing) => *existing = door,
            None => map.doors.push(door),
        }

        self.persist(&map)?;
        info!("Added door to map {map_id}");

        Ok(())
    }

    /// Applies a partial update to a placed door. Returns `None` if the map or
    /// the door does not exist.
    pub fn update_door(
        &self,
        map_id: &str,
        door_id: &str,
        update: UpdateMapDoorRequest,
    ) -> Result<Option<MapDoor>> {
        // A missing map has no doors, so there is nothing to update.
        if self.db.get(map_id)?.is_none() {
            return Ok(None);
        }

        let mut map = self.get_map(map_id)?;

        let Some(door) = map.doors.iter_mut().find(|door| door.id == door_id) else {
            return Ok(None);
        };

        if let Some(start) = update.start {
            door.start = start;
        }

        if let Some(end) = update.end {
            door.end = end;
        }

        if let Some(swing) = update.swing {
            door.swing = swing;
        }

        let updated = door.clone();
        self.persist(&map)?;
        info!("Updated door {door_id} on map {map_id}");

        Ok(Some(updated))
    }

    /// Removes a placed door. Returns `None` if the map or the door does not
    /// exist.
    pub fn remove_door(&self, map_id: &str, door_id: &str) -> Result<Option<()>> {
        // A missing map has no doors, so there is nothing to remove.
        if self.db.get(map_id)?.is_none() {
            return Ok(None);
        }

        let mut map = self.get_map(map_id)?;

        let original_len = map.doors.len();
        map.doors.retain(|door| door.id != door_id);

        if map.doors.len() == original_len {
            return Ok(None);
        }

        self.persist(&map)?;
        info!("Removed door {door_id} from map {map_id}");

        Ok(Some(()))
    }
}
