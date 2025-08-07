use shield_models::Camera;
use std::collections::HashMap;

/// Groups cameras by their tags, returning both grouped cameras and untagged ones
pub fn group_cameras_by_tags(cameras: &[Camera]) -> (HashMap<String, Vec<&Camera>>, Vec<&Camera>) {
    let mut tag_groups = HashMap::new();
    let mut untagged_cameras = Vec::new();

    for camera in cameras {
        if camera.tags.is_empty() {
            untagged_cameras.push(camera);
        } else {
            for tag in &camera.tags {
                tag_groups
                    .entry(tag.clone())
                    .or_insert_with(Vec::new)
                    .push(camera);
            }
        }
    }

    (tag_groups, untagged_cameras)
}

/// Gets camera names by their IDs
pub fn get_camera_names_by_ids(cameras: &[Camera], camera_ids: &[String]) -> Vec<String> {
    camera_ids
        .iter()
        .filter_map(|id| {
            cameras
                .iter()
                .find(|camera| &camera.id == id)
                .map(|camera| camera.name.clone())
        })
        .collect()
}

/// Gets camera IDs from a group of cameras
pub fn get_camera_ids(cameras: &[&Camera]) -> Vec<String> {
    cameras.iter().map(|camera| camera.id.clone()).collect()
}
