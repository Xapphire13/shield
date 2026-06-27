use serde::{Deserialize, Serialize};

/// A property map / scene. v1 holds only placed cameras; vector floor-plan
/// elements (walls, rooms — scalable/stylable) come later.
///
/// Coordinates are real-world (identity scale) measured in centimeters. The
/// outer map bounds are not modeled here — they are computed client-side
/// (bounding box of placed elements + buffer) for the minimap only.
#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Map {
    /// Unique identifier for the map.
    pub id: String,
    /// Human-readable name of the map.
    pub name: String,
    /// Display unit system for measurements. Internal storage is always
    /// centimeters; this only affects how values are shown/entered.
    pub unit_system: UnitSystem,
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
    /// Position of the camera in real-world coordinates (centimeters).
    pub position: Point,
    /// Field-of-view cone for the camera.
    pub fov: FieldOfView,
}

/// A point in the map's real-world coordinate space, in centimeters. Signed,
/// since objects may sit left of / above the origin.
#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Point {
    /// Horizontal coordinate, in centimeters.
    pub x: i32,
    /// Vertical coordinate, in centimeters.
    pub y: i32,
}

/// Field-of-view cone. Bearing is true-North relative, measured clockwise, in
/// whole degrees.
#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FieldOfView {
    /// Bearing the camera is aimed at, in whole degrees (0..360). 0 = North,
    /// clockwise.
    pub direction_deg: u16,
    /// Cone width, in whole degrees (0..360).
    pub angle_deg: u16,
    /// Cone length (range), in centimeters.
    pub range: i32,
}

/// Display unit system for measurements. Internal coordinates are always
/// centimeters; this only affects how values are shown/entered.
#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum UnitSystem {
    Metric,
    Imperial,
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
