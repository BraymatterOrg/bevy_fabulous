use std::any::TypeId;

use bevy::{
    ecs::system::{SystemId, SystemState},
    prelude::*,
    scene::SceneInstance,
};

use crate::{FabManager, FabTarget};

/// Whenever a scene handle is added to an entity consult the fab manager
/// and add a postfab if found. Postfabs are 'read-only' and can probably be
/// replaced with a reference/HashMap lookup so we don't have to worry about the performance of
/// a copy.
pub fn add_postfabs_to_spawned_scene(
    spawned_scenes: Query<(Entity, &Handle<Scene>), Added<Handle<Scene>>>,
    fab_manager: Res<FabManager>,
    mut cmds: Commands,
) {
    for (entity, spawned_scene) in spawned_scenes.iter() {
        let Some(postfab) = fab_manager.postfabs.get(spawned_scene) else {
            continue;
        };

        let Some(mut entcmds) = cmds.get_entity(entity) else {
            warn!("Could not get entity with added scene in Postfab system");
            continue;
        };

        entcmds.insert(postfab.clone());
    }
}

/// Iterates over all of the postfabs in the world, if there is a SceneInstance attached apply the hook
pub fn handle_scene_postfabs(world: &mut World) {
    let mut system_state = SystemState::<(
        Query<(Entity, &PostFab, &SceneInstance)>,
        Query<&Children>,
        Res<SceneSpawner>,
    )>::new(world);
    let (postfabs, children, scene_spawner) = system_state.get(world);

    let mut pipes_to_run = vec![];
    let mut root_entities = vec![];
    //For every entity with a postfab
    for (entity, postfab, instance) in postfabs.iter() {
        if !scene_spawner.instance_is_ready(**instance) {
            continue;
        }

        root_entities.push(entity);

        //Iterate over all of a postfabs pipe
        for pipe in postfab.pipes.iter() {
            //Attempt to apply to the parent, then any children
            'child: for applicable_entity in
                std::iter::once(entity).chain(children.iter_descendants(entity))
            {
                let Some(ent) = world.get_entity(applicable_entity) else {
                    warn!("Could not get entity for postfab, aborting postfab");
                    continue;
                };
                //Check if enity has required Name

                match ent.get::<Name>() {
                    Some(n) => {
                        if !pipe.name_criteria.eval(n) {
                            continue 'child;
                        }
                    }
                    None => {
                        continue 'child;
                    }
                }

                //Check if entity has required components
                for t in &pipe.with_components {
                    if !ent.contains_type_id(*t) {
                        continue 'child;
                    }
                }

                //Check if entity does not have components
                for t in &pipe.without_components {
                    if ent.contains_type_id(*t) {
                        continue 'child;
                    }
                }

                //Run System
                pipes_to_run.push((pipe.system, applicable_entity));
            }
        }
    }

    //Remove the postfab for the parent so it's not processed again
    for ent in root_entities {
        world.entity_mut(ent).remove::<PostFab>();
    }

    // Run the system with the entity as the input
    for (system, ent) in pipes_to_run {
        if let Err(e) = world.run_system_with_input(system, ent) {
            error!("Error running system for postfab pipe!\n {}", e);
        }
    }
}

/// Postfabs are used to modify a scene every time it's spawned
/// You may use these to read component data and attach contextual components to entities
/// of spawning such as changing the material color based on health / faction etc.
/// These run every time you spawn the PostFab
#[derive(Clone, Component)]
pub struct PostFab {
    pub scene: FabTarget,
    pub pipes: Vec<PostfabPipe>,
}

/// An individual element of a postfab. Postfabs contain an ordered collection of pipes that run
/// in order. The pipe has various filtering functions etc. to make this easier. These filters could/should
/// be copied over to the prefab behavior as well
#[derive(Clone)]
pub struct PostfabPipe {
    pub system: SystemId<Entity, ()>,
    /// Only apply pipe to entities with the following components
    pub with_components: Vec<TypeId>,
    /// Only apply pipe to entities without the following components
    pub without_components: Vec<TypeId>,
    /// Only apply pipe to entities matching name criteria
    pub name_criteria: NameCriteria,
    /// Only apply pipe to the scene root entity
    pub root_only: bool,
}

impl PostfabPipe {
    /// Get a new pipeline builder
    pub fn new(system: SystemId<Entity, ()>) -> Self {
        Self {
            system,
            with_components: vec![],
            without_components: vec![],
            name_criteria: NameCriteria::Any,
            root_only: false,
        }
    }

    /// Apply only to entities with the following components
    pub fn with_components(mut self, components: Vec<TypeId>) -> Self {
        self.with_components = components;
        self
    }

    /// Apply only to entities without the following components
    pub fn without_components(mut self, components: Vec<TypeId>) -> Self {
        self.without_components = components;
        self
    }

    /// Apply only to entities in the scene/root with a name equal to the input
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name_criteria = NameCriteria::Equals(name.into());
        self
    }

    /// Apply only to entities in the scene/root with a name containing the input
    pub fn name_contains(mut self, name: impl Into<String>) -> Self {
        self.name_criteria = NameCriteria::Contains(name.into());
        self
    }

    /// Apply only to entities in the scene/root with a name starting with the input
    pub fn name_starts_with(mut self, name: impl Into<String>) -> Self {
        self.name_criteria = NameCriteria::StartsWith(name.into());
        self
    }

    /// Apply only to entities in the scene/root with a name ending with the input
    pub fn name_ends_with(mut self, name: impl Into<String>) -> Self {
        self.name_criteria = NameCriteria::EndsWith(name.into());
        self
    }
    
    /// Whether this applies to the scene root only
    pub fn root_only(mut self) -> Self {
        self.root_only = true;
        self
    }
}

/// Name component criteria for determining whether a pipe should run on a given entity
#[derive(Clone)]
pub enum NameCriteria {
    Any,
    Equals(String),
    Contains(String),
    StartsWith(String),
    EndsWith(String),
}

impl NameCriteria {
    pub fn eval(&self, name: &Name) -> bool {
        match self {
            NameCriteria::Any => true,
            NameCriteria::Equals(c) => c == &name.to_string(),
            NameCriteria::Contains(c) => name.to_string().contains(c.as_str()),
            NameCriteria::StartsWith(c) => name.starts_with(c.as_str()),
            NameCriteria::EndsWith(c) => name.ends_with(c.as_str()),
        }
    }
}
