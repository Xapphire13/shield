use std::collections::HashMap;

use dioxus::prelude::*;

use crate::components::{Camera, ui::RowGroup};

mod components;

const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let cameras = use_resource(|| async {
        let hostname = web_sys::window().unwrap().location().hostname().unwrap();
        let url = format!("http://{hostname}:3000/cameras");

        reqwest::get(url)
            .await
            .unwrap()
            .json::<Vec<shield_models::Camera>>()
            .await
            .unwrap()
    });
    let cameras = cameras.cloned().unwrap_or_else(|| vec![]);
    let mut tag_groups: HashMap<String, Vec<&shield_models::Camera>> = HashMap::new();
    let mut untagged_cameras = vec![];

    for camera in cameras.iter() {
        for tag in &camera.tags {
            tag_groups
                .entry(tag.to_owned())
                .and_modify(|group| {
                    group.push(camera);
                })
                .or_insert_with(|| vec![camera]);
        }

        if camera.tags.is_empty() {
            untagged_cameras.push(camera);
        }
    }

    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        {tag_groups.iter().map(|(tag, cameras)| rsx! {
            RowGroup { label: tag,
                {cameras.iter().map(|&camera| rsx! {
                    Camera { camera: camera.clone() }
                })}
            }
        })}

        if !untagged_cameras.is_empty() {
            RowGroup { label: "Untagged",
                {untagged_cameras.iter().map(|&camera| rsx! {
                    Camera { camera: camera.clone() }
                })}
            }
        }

        {dioxus_feather_icons::sprite!()}
    }
}
