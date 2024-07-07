use bevy::{ecs::system::BoxedSystem, prelude::*};

use crate::FabManager;

/// Apply pipes to  the loaded Scene
pub fn apply_pipes_to_loaded_scene(
    asset_server: Res<AssetServer>,
    mut events: EventReader<AssetEvent<Scene>>,
    mut scenes: ResMut<Assets<Scene>>,
    mut prefabs: ResMut<FabManager>,
) {
    // Go over all events
    for event in events.read() {
        // Only when an asset is added
        let AssetEvent::LoadedWithDependencies { id } = event else {
            continue;
        };

        //Get the path of the asset
        let Some(asset_path) = asset_server.get_path(id.untyped()) else {
            debug!("Could not get asset path for asset! {}", id);
            continue;
        };
        let path = asset_path.to_string();

        //Get the prefab from the manager if it exists
        let Some(prefab) = prefabs.loaded_prefab_mut(path.clone()) else {
            continue;
        };

        debug!("Found prefab definition for loaded asset: {}", path);

        //Get the  Scene
        let Some(scene_handle) = asset_server.get_handle::<Scene>(path.clone()) else {
            warn!(
                "Could not get scene for asset path :{} from asset server",
                path
            );
            continue;
        };

        let Some(scene) = scenes.get_mut(scene_handle.id()) else {
            warn!("Could not get scene from gltf from Asset<Scene>");
            continue;
        };

        // Apply all pipes to the scene
        for pipe in prefab.pipeline.iter_mut() {
            pipe.apply(&mut scene.world);
        }
    }
}

/// Applies ScenePipes to the loaded scene `World`
pub struct Prefab {
    /// The path to the asset on the filesystem
    pub path: String,

    /// Pipes to run on load
    pub pipeline: Vec<Box<dyn PrefabPipe + Send + Sync>>,
}

impl Prefab {
    /// Create a new, prefab based on a scene with no modifications
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            pipeline: vec![],
        }
    }

    /// Add a step to the prefab's pipeline
    pub fn with_pipe<T: PrefabPipe + Send + Sync + 'static>(mut self, pipe: T) -> Self {
        self.pipeline.push(Box::new(pipe));
        self
    }

    /// Add multiple steps of the same kind to a prefab's pipeline
    pub fn with_pipes<T: PrefabPipe + Send + Sync + 'static>(mut self, pipes: Vec<T>) -> Self {
        for pipe in pipes {
            self.pipeline.push(Box::new(pipe));
        }

        self
    }

    /// Add a **System** as a pipeline step. Internally registers the system to the scene world, runs, and deletes the SystemId entity
    pub fn with_system<M>(
        mut self,
        sys: impl IntoSystem<(), (), M> + Send + Sync + 'static + Copy,
    ) -> Self {
        self = self.with_pipe(Self::system(sys));

        self
    }

    /// Cursed Trait Boxing magic to make a nice API for you UwU
    fn system<M, T: IntoSystem<(), (), M> + Send + Sync + 'static + Copy>(
        a: T,
    ) -> Box<dyn FnMut() -> BoxedSystem + Send + Sync> {
        Box::new(move || Box::new(IntoSystem::into_system(a)) as BoxedSystem)
    }
}

///Used to transform a scene, but avoid Transform as a term - it's already overloaded
pub trait PrefabPipe {
    // Applies the pipe to the entity
    fn apply(&mut self, world: &mut World);
}

impl<T: FnMut() -> BoxedSystem + Send + Sync> PrefabPipe for T {
    fn apply(&mut self, world: &mut World) {
        let sys = self();
        let sys_id = world.register_boxed_system(sys);
        let _ = world.run_system(sys_id);
        world.despawn(sys_id.entity());
        world.flush();
    }
}