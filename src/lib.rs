use bevy::{
    ecs::{
        system::{EntityCommand, EntityCommands, SystemParam, SystemState},
        world::Command,
    },
    prelude::*,
    utils::HashMap,
};
use postfab::{
    add_postfabs_to_spawned_scene, handle_scene_postfabs, PostFab, PostFabVariant, PostfabPipe,
};
use prefab::{apply_pipes_to_loaded_scene, Prefab};

pub mod materials;
pub mod postfab;
pub mod prefab;
pub mod prelude;

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
        match &prefab.target {
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

impl From<Handle<Gltf>> for FabTarget {
    fn from(value: Handle<Gltf>) -> Self {
        Self::Gltf(value)
    }
}

impl From<Handle<Scene>> for FabTarget {
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

    for (gltf_handle, _fab) in fabs.prefab_gltfs.iter() {
        if asset_server.is_loaded_with_dependencies(gltf_handle) {
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

#[derive(Default)]
pub struct GltfScene {
    pub handle: Handle<Gltf>,
    pub scene_idx: usize,
    pub location: Transform,
    //If present scene will be spawned 'into' the provided entity
    pub entity: Option<Entity>
}

impl GltfScene {
    pub fn new(gltf: Handle<Gltf>) -> Self {
        Self {
            handle: gltf,
            ..default()
        }
    }

    pub fn with_bundle<B: Bundle>(self, bundle: B) -> SpawnGltfScene<B> {
        SpawnGltfScene {
            bundle: Some(bundle),
            gltf: self.handle,
            scene_idx: self.scene_idx,
            location: self.location,
            entity: self.entity
        }
    }

    pub fn build(self) -> SpawnGltfScene<()> {
        SpawnGltfScene {
            bundle: None,
            gltf: self.handle,
            scene_idx: self.scene_idx,
            location: self.location,
            entity: self.entity,
        }
    }

    pub fn with_scene(self, scene: usize) -> Self {
        Self {
            scene_idx: scene,
            ..self
        }
    }

    pub fn at_location(mut self, t: Transform) -> Self {
        self.location = t;
        self
    }

    pub fn into_entity(mut self, entity: Entity) -> Self {
        self.entity = Some(entity);
        self
    }
}

#[derive(Clone)]
pub struct SpawnGltfScene<B: Bundle> {
    pub gltf: Handle<Gltf>,
    pub scene_idx: usize,
    pub location: Transform,
    pub bundle: Option<B>,

    //If present will attach to the provided entity instead
    pub entity: Option<Entity>,
}

impl<B: Bundle> SpawnGltfScene<B> {
    pub fn with_bundle(mut self, bundle: B) -> Self {
        self.bundle = Some(bundle);
        self
    }

    pub fn with_scene(self, scene: usize) -> Self {
        Self {
            scene_idx: scene,
            ..self
        }
    }

    pub fn at_location(mut self, t: Transform) -> Self {
        self.location = t;
        self
    }
}

impl<B: Bundle> Command for SpawnGltfScene<B> {
    fn apply(self, world: &mut World) {
        let mut sys_state = SystemState::<(Res<Assets<Gltf>>, Commands)>::new(world);
        let (gltfs, mut cmds) = sys_state.get(world);

        let Some(gltf) = gltfs.get(&self.gltf) else {
            warn!("Could not get GLTF for SpawnGltfScene");
            return;
        };

        let Some(scene) = gltf.scenes.get(self.scene_idx) else {
            warn!(
                "Could not find scene at index {} to spawn gltf scene",
                self.scene_idx
            );
            return;
        };

        let mut spawned_scene = cmds.spawn((SceneRoot(scene.clone()), self.location));

        if let Some(bundle) = self.bundle {
            spawned_scene.insert(bundle);
        }

        sys_state.apply(world);
    }
}

pub struct SpawnPostfabVariant<B: Bundle> {
    pub scene: SpawnGltfScene<B>,
    pub variance: PostFabVariant,
}

impl<B: Bundle> Command for SpawnPostfabVariant<B> {
    fn apply(self, world: &mut World) {
        let mut sys_state = SystemState::<(Res<Assets<Gltf>>, Commands)>::new(world);
        let (gltfs, mut cmds) = sys_state.get(world);

        let Some(gltf) = gltfs.get(&self.scene.gltf) else {
            warn!("Could not get GLTF for SpawnGltfScene");
            return;
        };

        let Some(scene) = gltf.scenes.get(self.scene.scene_idx) else {
            warn!(
                "Could not find scene at index {} to spawn gltf scene",
                self.scene.scene_idx
            );
            return;
        };

        let mut entcmds = match self.scene.entity {
            Some(entity) => cmds.entity(entity),
            None => cmds.spawn_empty(),
        };

        let spawned_scene = entcmds.insert((SceneRoot(scene.clone()), self.scene.location));

        if let Some(bundle) = self.scene.bundle {
            spawned_scene.insert((bundle, self.variance));
        } else {
            spawned_scene.insert(self.variance);
        }

        sys_state.apply(world);
    }
}

pub trait SpawnGltfCmdExt {
    fn spawn_gltf<T: Into<SpawnGltfScene<B>>, B: Bundle>(&mut self, cmd: T) -> Entity;
    fn spawn_gltf_variant<T: Into<SpawnGltfScene<B>>, B: Bundle, V: Into<Vec<PostfabPipe>>>(
        &mut self,
        scene: T,
        variance: V,
    ) -> Entity;
}

impl SpawnGltfCmdExt for Commands<'_, '_> {
    fn spawn_gltf<T: Into<SpawnGltfScene<B>>, B: Bundle>(&mut self, cmd: T) -> Entity {
        
        let mut spawn_gltf: SpawnGltfScene<B> = cmd.into(); 

        match spawn_gltf.entity {
            Some(ent) => {
                self.queue(spawn_gltf);
                ent
            },
            None => {
                let id = self.spawn_empty().id();
                spawn_gltf.entity = Some(id);
                self.queue(spawn_gltf);
                id
            },
        }
    }

    fn spawn_gltf_variant<T: Into<SpawnGltfScene<B>>, B: Bundle, V: Into<Vec<PostfabPipe>>>(
        &mut self,
        scene: T,
        variance: V,
    ) -> Entity {
        let mut scene: SpawnGltfScene<B> = scene.into();
        match scene.entity {
            Some(ent) => {
                self.queue(SpawnPostfabVariant{
                    scene,
                    variance: PostFabVariant::from(variance.into())
                });
                ent
            },
            None => {
                let id = self.spawn_empty().id();
                scene.entity = Some(id);
                self.queue(SpawnPostfabVariant {
                    scene,
                    variance: PostFabVariant::from(variance.into()),
                });
        
                id
            },
        }
    }
}

impl SpawnGltfCmdExt for ChildBuilder<'_> {
    fn spawn_gltf<T: Into<SpawnGltfScene<B>>, B: Bundle>(&mut self, cmd: T) -> Entity {
        let mut spawn_gltf: SpawnGltfScene<B> = cmd.into(); 

        match spawn_gltf.entity {
            Some(ent) => {
                self.enqueue_command(spawn_gltf);
                ent
            },
            None => {
                let id = self.spawn_empty().id();
                spawn_gltf.entity = Some(id);
                self.enqueue_command(spawn_gltf);
                id
            },
        }
    }

    fn spawn_gltf_variant<T: Into<SpawnGltfScene<B>>, B: Bundle, V: Into<Vec<PostfabPipe>>>(
        &mut self,
        scene: T,
        variance: V,
    ) -> Entity {
        let mut scene: SpawnGltfScene<B> = scene.into();
        match scene.entity {
            Some(ent) => {
                self.enqueue_command(SpawnPostfabVariant{
                    scene,
                    variance: PostFabVariant::from(variance.into())
                });
                ent
            },
            None => {
                let id = self.spawn_empty().id();
                scene.entity = Some(id);
                self.enqueue_command(SpawnPostfabVariant {
                    scene,
                    variance: PostFabVariant::from(variance.into()),
                });
        
                id
            },
        }
    }
}


    /// For trait objects of commands, to be used where generics cannot
    pub trait DynCommand: Send + Sync {
        fn dyn_add(self: Box<Self>, cmd: &mut Commands);
        fn dyn_apply(self: Box<Self>, world: &mut World);
        fn dyn_clone(&self) -> Box<dyn DynCommand>;
    }

    impl<T: Command + Clone + Sync> DynCommand for T {
        fn dyn_add(self: Box<Self>, cmd: &mut Commands) {
            cmd.queue(*self);
        }

        fn dyn_apply(self: Box<Self>, world: &mut World) {
            self.apply(world);
        }

        fn dyn_clone(&self) -> Box<dyn DynCommand> {
            Box::new(self.clone())
        }
    }

    impl Clone for Box<dyn DynCommand> {
        fn clone(&self) -> Self {
            self.dyn_clone()
        }
    }

    /// For trait objects of commands, to be used where generics cannot
    pub trait DynEntityCommand: Send + Sync {
        fn dyn_add(self: Box<Self>, cmd: &mut EntityCommands);
        fn dyn_clone(&self) -> Box<dyn DynEntityCommand>;
    }

    impl<T: EntityCommand + Clone + Sync> DynEntityCommand for T {
        fn dyn_add(self: Box<Self>, cmd: &mut EntityCommands) {
            cmd.queue(*self);
        }

        fn dyn_clone(&self) -> Box<dyn DynEntityCommand> {
            Box::new(self.clone())
        }
    }

    impl Clone for Box<dyn DynEntityCommand> {
        fn clone(&self) -> Self {
            self.dyn_clone()
        }
    }
