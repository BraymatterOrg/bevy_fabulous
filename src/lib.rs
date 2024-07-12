use bevy::{ecs::system::SystemParam, prelude::*, utils::HashMap};
use postfab::{add_postfabs_to_spawned_scene, handle_scene_postfabs, PostFab};
use prefab::{apply_pipes_to_loaded_scene, Prefab};

pub mod materials;
pub mod postfab;
pub mod prefab;

pub struct FabulousPlugin;

impl Plugin for FabulousPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FabManager>();
        app.add_systems(
            PreUpdate,
            (
                apply_pipes_to_loaded_scene,
                convert_gltf_postfabs_to_scene,
                add_postfabs_to_spawned_scene,
                handle_scene_postfabs,
            )
                .chain(),
        );
    }
}

#[derive(Resource, Default)]
pub struct FabManager {
    pub prefabs: HashMap<String, Prefab>,
    pub postfabs: HashMap<Handle<Scene>, PostFab>,
    /// When a scene is part of a gltf, store them here to be processed once the scene is loaded
    postfab_gltfs: HashMap<Handle<Gltf>, PostFab>,
}

impl FabManager {
    pub fn register_prefab(&mut self, prefab: Prefab) {
        self.prefabs.insert(prefab.path.clone(), prefab);
    }

    pub fn loaded_prefab(&self, path: impl Into<String>) -> Option<&Prefab> {
        let string_path: String = path.into();
        let prefab = self.prefabs.iter().find(|(sourcepath, _fab)| {
            if string_path == **sourcepath {
                return true;
            }

            false
        });

        if let Some(p) = prefab {
            return Some(p.1);
        }

        None
    }

    pub fn loaded_prefab_mut(&mut self, path: impl Into<String>) -> Option<&mut Prefab> {
        let string_path: String = path.into();
        let prefab = self.prefabs.iter_mut().find(|(sourcepath, _fab)| {
            if string_path == **sourcepath {
                return true;
            }

            false
        });

        if let Some(p) = prefab {
            return Some(p.1);
        }

        None
    }

    pub fn register_postfab(&mut self, postfab: PostFab) {
        match &postfab.scene {
            postfab::PostFabTarget::Scene(scene) => {
                self.postfabs.insert(scene.clone(), postfab);
            }
            postfab::PostFabTarget::Gltf(gltf) => {
                self.postfab_gltfs.insert(gltf.clone(), postfab);
            }
        }
    }
}

#[derive(SystemParam)]
pub struct PostFabRegistrationParams<'w> {
    pub gltfs: Res<'w, Assets<Gltf>>,
    pub scenes: Res<'w, Assets<Scene>>,
}

fn convert_gltf_postfabs_to_scene(
    asset_server: Res<AssetServer>,
    postfab_params: PostFabRegistrationParams,
    mut fabs: ResMut<FabManager>,
) {
    let mut loaded_fabs = vec![];

    for (gltf_handle, fab) in fabs.postfab_gltfs.iter() {
        if asset_server.is_loaded_with_dependencies(gltf_handle) {
            loaded_fabs.push((gltf_handle.clone(), fab.clone()));
        }
    }

    for (handle, _fab) in loaded_fabs {
        let Some(fab) = fabs.postfab_gltfs.remove(&handle) else {
            warn!("Found gltf postfab loaded, but could not find it in fabs.postfab map!");
            continue;
        };

        let Some(gltf) = postfab_params.gltfs.get(&handle) else {
            warn!("Could not find gltf handle for postfab!");
            continue;
        };

        let Some(scene) = gltf.scenes.first() else {
            warn!("Attempted to create postfab with a gltf containing no scenes!");
            continue;
        };

        info!("Converting GLTF Postfab To Scene!");
        fabs.postfabs.insert(scene.clone(), fab);
    }
}
