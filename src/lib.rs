use std::marker::PhantomData;

use bevy::{
    ecs::{
        component::StorageType,
        system::BoxedSystem,
        world::{Command, DeferredWorld},
    },
    prelude::*,
    utils::HashMap,
};

pub struct FabulousPlugin;

impl Plugin for FabulousPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FabManager>();
        app.add_systems(PreUpdate, (apply_pipes_to_loaded_scene,));
    }
}

/// Apply pipes to  the loaded Scene
fn apply_pipes_to_loaded_scene(
    asset_server: Res<AssetServer>,
    mut events: EventReader<AssetEvent<Gltf>>,
    gltfs: ResMut<Assets<Gltf>>,
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

        //Get the GLTF and Scene
        let Some(mut gltf_handle) = asset_server.get_handle::<Gltf>(path.clone()) else {
            warn!(
                "Could not get gltf handle for asset path :{} from asset server",
                path
            );
            continue;
        };

        let Some(gltf) = gltfs.get(&mut gltf_handle) else {
            warn!("Could not get gltf asset from Assets<gltf>");
            continue;
        };

        //NOTE: Only works with the first scene in the gltf!
        let Some(scene_handle) = gltf.scenes.first() else {
            warn!("No scenes in GLTF LoadedPrefab scene");
            continue;
        };

        let Some(scene) = scenes.get_mut(scene_handle) else {
            warn!("Could not get scene from gltf from Asset<Scene>");
            continue;
        };

        // Apply all pipes to the scene
        for pipe in prefab.node_pipe.iter_mut() {
            pipe.apply(&mut scene.world);
        }
    }
}

#[derive(Resource, Default)]
pub struct FabManager {
    pub prefabs: HashMap<String, Prefab>,
    pub postfabs: HashMap<Handle<Scene>, PostFab>,
}

impl FabManager {
    pub fn register_loaded_prefab(&mut self, prefab: Prefab) {
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

    pub fn register_runtime_prefab(&mut self, prefab: PostFab) {
        self.postfabs.insert(prefab.scene.clone(), prefab);
    }
}

/// Applies ScenePipes to the loaded scene `World`
pub struct Prefab {
    /// The path to the asset on the filesystem
    pub path: String,

    /// Pipes to run on load
    pub node_pipe: Vec<Box<dyn PrefabPipe + Send + Sync>>,
}

impl Prefab {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            node_pipe: vec![],
        }
    }

    pub fn with_pipe<T: PrefabPipe + Send + Sync + 'static>(mut self, pipe: T) -> Self {
        self.node_pipe.push(Box::new(pipe));
        self
    }

    pub fn with_pipes<T: PrefabPipe + Send + Sync + 'static>(mut self, pipes: Vec<T>) -> Self {
        for pipe in pipes{
            self.node_pipe.push(Box::new(pipe));
        }
        
        self
    }
}

///Used to transform a scene, but avoid Transform as a term - it's already overloaded
pub trait PrefabPipe {
    // Applies the pipe to the entity
    fn apply(&mut self, world: &mut World);
}

impl<T: Fn() -> BoxedSystem + Send + Sync> PrefabPipe for T {
    fn apply(&mut self, world: &mut World) {
        let sys = self();
        let sys_id = world.register_boxed_system(sys);
        let _ = world.run_system(sys_id);
        world.despawn(sys_id.entity());
    }
}

/// Applies ScenePipes to an entity after it's been spawned
pub struct PostFab {
    pub scene: Handle<Scene>,
    pub pipes: Vec<Box<dyn PostfabPipe + Send + Sync>>,
}

impl Component for PostFab {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut bevy::ecs::component::ComponentHooks) {
        hooks.on_add(|mut world, targeted_entity, _component_id| {
            let Some(mut entmut) = world.get_entity_mut(targeted_entity) else {
                warn!("Could not get postfab def");
                return;
            };

            let Some(fabdef) = entmut.get_mut::<PostFab>() else {
                warn!("Could not get fabdef from entity in hook");
                return;
            };

            let pipes: Vec<Box<dyn PostfabPipe + Send + Sync>> =
                fabdef.pipes.iter().map(|p| p.postfab_clone()).collect();

            for mut pipe in pipes {
                pipe.apply(&mut world)
            }

            world
                .commands()
                .add(RemoveComponent::<PostFab>::new(targeted_entity));
        });
    }
}

pub trait PostfabPipe {
    fn apply(&mut self, world: &mut DeferredWorld);
    fn postfab_clone(&self) -> Box<dyn PostfabPipe + Send + Sync>;
}

pub struct RemoveComponent<T: Component> {
    ent: Entity,
    _phantom_data: PhantomData<T>,
}

impl<T: Component> Command for RemoveComponent<T> {
    fn apply(self, world: &mut World) {
        let Some(mut entmut) = world.get_entity_mut(self.ent) else {
            warn!("Could not get entity to remove component");
            return;
        };

        entmut.remove::<T>();
    }
}

impl<T: Component> RemoveComponent<T> {
    pub fn new(ent: Entity) -> Self {
        Self {
            ent,
            _phantom_data: PhantomData,
        }
    }
}

pub fn pipe(
    thing: impl Fn() -> BoxedSystem + Send + Sync + 'static,
) -> Box<dyn PrefabPipe + Send + Sync> {
    Box::new(thing) as Box<dyn PrefabPipe + Send + Sync>
}
