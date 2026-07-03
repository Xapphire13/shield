use serde::{Deserialize, Serialize};

/// A property map / scene: placed cameras plus vector floor-plan elements
/// (walls, doors). Other floor-plan elements (e.g. rooms) may still come
/// later.
///
/// Coordinates are real-world (identity scale) measured in centimeters. The
/// outer map bounds are not modeled here — they are computed client-side
/// (bounding box of placed elements + buffer) for the minimap only. Display
/// units (metric/imperial) are a per-user client-side preference, out of scope
/// for this shared model.
#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Map {
    /// Unique identifier for the map.
    pub id: String,
    /// Human-readable name of the map.
    pub name: String,
    /// Cameras placed on the map.
    pub cameras: Vec<MapCamera>,
    /// Walls (and fences) placed on the map.
    pub walls: Vec<MapWall>,
    /// Doors (and gates) placed on the map.
    pub doors: Vec<MapDoor>,
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

/// A wall (also used to represent a fence): a connected polyline in
/// real-world coordinates. In practice always has >= 2 vertices.
#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MapWall {
    /// Unique identifier for the wall.
    pub id: String,
    /// Polyline vertices; segment `i` runs from `vertices[i]` to
    /// `vertices[i + 1]`.
    pub vertices: Vec<Point>,
    /// If true, an implicit closing segment runs from the last vertex back to
    /// `vertices[0]`. Kept as a flag rather than a duplicated final vertex so
    /// segment indices stay unambiguous and there's no degenerate
    /// duplicate-point edge case.
    pub closed: bool,
    /// Display color, drawn from a curated palette.
    pub color: WallColor,
}

/// A curated, closed color palette for walls/fences (not an arbitrary hex
/// value), so the UI can offer a fixed swatch picker and the server can
/// validate for free.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub enum WallColor {
    #[default]
    Slate,
    Clay,
    Moss,
    Amber,
    Sky,
    Rose,
}

/// A door (also used for a gate): an independent two-point element, not
/// attached to any wall by reference. Doors are freestanding, placed by the
/// user in a gap left while drawing wall sections — the same way a camera is
/// a freestanding placed element.
#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MapDoor {
    /// Unique identifier for the door.
    pub id: String,
    /// One end of the door opening, in real-world coordinates.
    pub start: Point,
    /// The other end of the door opening, in real-world coordinates. Width is
    /// implicit: the distance between `start` and `end`.
    pub end: Point,
    /// Which side the door opens toward.
    pub swing: DoorSwing,
}

/// Which side a door/gate opens toward. Deliberately a plain 2-value enum
/// rather than an angle — only the opening side needs to be visualized, not
/// an exact sweep angle.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub enum DoorSwing {
    #[default]
    Left,
    Right,
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

/// Partial update for a placed camera; omitted fields are left unchanged.
#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMapCameraRequest {
    /// New position, if being changed.
    pub position: Option<Point>,
    /// New field-of-view cone, if being changed.
    pub fov: Option<FieldOfView>,
}

/// Partial update for a placed wall; omitted fields are left unchanged.
#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMapWallRequest {
    /// New vertices, if being changed. Replaces the whole vector — not a
    /// per-index patch — mirroring how `UpdateMapCameraRequest` replaces
    /// `position`/`fov` wholesale.
    pub vertices: Option<Vec<Point>>,
    /// New closed flag, if being changed.
    pub closed: Option<bool>,
    /// New color, if being changed.
    pub color: Option<WallColor>,
}

/// Partial update for a placed door; omitted fields are left unchanged.
#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMapDoorRequest {
    /// New start point, if being changed.
    pub start: Option<Point>,
    /// New end point, if being changed.
    pub end: Option<Point>,
    /// New swing side, if being changed.
    pub swing: Option<DoorSwing>,
}
