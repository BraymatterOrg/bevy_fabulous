use bevy::{
    prelude::*,
    utils::HashMap,
};
use postfab::PostFab;
use prefab::{apply_pipes_to_loaded_scene, Prefab};

pub mod postfab;
pub mod prefab;

pub struct FabulousPlugin;

impl Plugin for FabulousPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FabManager>();
        app.add_systems(PreUpdate, (apply_pipes_to_loaded_scene,));
    }
}

#[derive(Resource, Default)]
pub struct FabManager {
    pub prefabs: HashMap<String, Prefab>,
    pub postfabs: HashMap<Handle<Scene>, PostFab>,
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

    pub fn register_postfab(&mut self, prefab: PostFab) {
        self.postfabs.insert(prefab.scene.clone(), prefab);
    }
}