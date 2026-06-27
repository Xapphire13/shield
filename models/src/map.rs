use serde::{Deserialize, Serialize};

/// A property map / scene. v1 holds only placed cameras; vector floor-plan
/// elements (walls, rooms — scalable/stylable) come later.
#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Map {
    /// Unique identifier for the map.
    pub id: String,
    /// Human-readable name of the map.
    pub name: String,
    /// Cameras placed on the map.
    pub cameras: Vec<MapCamera>,
}

/// A camera placed on the map. `camera_id` == `Camera.id`; camera metadata is
/// joined at read time, never stored here.
#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MapCamera {
    /// Identifier of the referenced camera (`== Camera.id`).
    pub camera_id: String,
    /// Position of the camera in the map's logical viewBox coordinate space.
    pub position: Point,
    /// Field-of-view cone for the camera.
    pub fov: FieldOfView,
}

/// A point in the map's logical viewBox coordinate space.
#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Point {
    /// Horizontal coordinate.
    pub x: f32,
    /// Vertical coordinate.
    pub y: f32,
}

/// Field-of-view cone. Bearing is true-North relative, measured clockwise.
#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FieldOfView {
    /// Bearing the camera is aimed at, in degrees. 0 = North, clockwise.
    pub direction_deg: f32,
    /// Cone width, in degrees.
    pub angle_deg: f32,
    /// Cone length, in viewBox units.
    pub range: f32,
}

/// Partial update for a placed camera; omitted fields are left unchanged.
#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMapCameraRequest {
    /// New position, if being changed.
    pub position: Option<Point>,
    /// New field-of-view cone, if being changed.
    pub fov: Option<FieldOfView>,
}
