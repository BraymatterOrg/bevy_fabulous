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
                convert_gltffabs_to_scenefabs,
                apply_pipes_to_loaded_scene,
                add_postfabs_to_spawned_scene,
                handle_scene_postfabs,
            )
                .chain(),
        );
    }
}

#[derive(Resource, Default)]
pub struct FabManager {
    pub prefabs: HashMap<Handle<Scene>, Prefab>,
    pub postfabs: HashMap<Handle<Scene>, PostFab>,
    /// When a scene is part of a gltf, store them here to be processed once the scene is loaded
    postfab_gltfs: HashMap<Handle<Gltf>, PostFab>,
    prefab_gltfs: HashMap<Handle<Gltf>, Prefab>,
}

impl FabManager {
    pub fn register_prefab(&mut self, prefab: Prefab) {
        match &prefab.target{
            FabTarget::Scene(scene) => self.prefabs.insert(scene.clone(), prefab),
            FabTarget::Gltf(gltf) => self.prefab_gltfs.insert(gltf.clone(), prefab),
        };
    }

    pub fn prefab(&self, scene: &Handle<Scene>) -> Option<&Prefab> {
        let prefab = self.prefabs.get(scene);

        if let Some(p) = prefab {
            return Some(p);
        }

        None
    }

    pub fn prefab_mut(&mut self, scene: &Handle<Scene>) -> Option<&mut Prefab> {
        let prefab = self.prefabs.get_mut(scene);

        if let Some(p) = prefab {
            return Some(p);
        }

        None
    }

    pub fn register_postfab(&mut self, postfab: PostFab) {
        match &postfab.scene {
            FabTarget::Scene(scene) => {
                self.postfabs.insert(scene.clone(), postfab);
            }
            
            FabTarget::Gltf(gltf) => {
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

#[derive(Clone)]
pub enum FabTarget {
    Scene(Handle<Scene>),
    Gltf(Handle<Gltf>),
}

impl From<Handle<Gltf>> for FabTarget{
    fn from(value: Handle<Gltf>) -> Self {
        Self::Gltf(value)
    }
}

impl From<Handle<Scene>> for FabTarget{
    fn from(value: Handle<Scene>) -> Self {
        Self::Scene(value)
    }
}

fn convert_gltffabs_to_scenefabs(
    asset_server: Res<AssetServer>,
    postfab_params: PostFabRegistrationParams,
    mut fabs: ResMut<FabManager>,
) {
    let mut loaded_postfabs = vec![];
    let mut loaded_prefabs = vec![];
    
    for (gltf_handle, _fab) in fabs.postfab_gltfs.iter() {
        if asset_server.is_loaded_with_dependencies(gltf_handle) {
            loaded_postfabs.push(gltf_handle.clone());
        }
    }

    for (gltf_handle, _fab) in fabs.prefab_gltfs.iter(){
        if asset_server.is_loaded_with_dependencies(gltf_handle){
            loaded_prefabs.push(gltf_handle.clone())
        }
    }
    
    for handle in loaded_postfabs {
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

        debug!("Converting GLTF Postfab To Scene!");
        fabs.postfabs.insert(scene.clone(), fab);
    }

    for handle in loaded_prefabs {
        let Some(fab) = fabs.prefab_gltfs.remove(&handle) else {
            warn!("Found gltf prefab loaded, but could not find it in fabs.prefab map!");
            continue;
        };

        let Some(gltf) = postfab_params.gltfs.get(&handle) else {
            warn!("Could not find gltf handle for prefab!");
            continue;
        };

        let Some(scene) = gltf.scenes.first() else {
            warn!("Attempted to create postfab with a gltf containing no scenes!");
            continue;
        };

        debug!("Converting GLTF Postfab To Scene!");
        fabs.prefabs.insert(scene.clone(), fab);
    }
}
