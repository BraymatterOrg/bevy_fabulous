use std::any::TypeId;

use bevy::{
    ecs::system::{SystemId, SystemState},
    prelude::*,
    scene::SceneInstance,
};

use crate::{DynCommand, DynEntityCommand, FabManager, FabTarget};

/// Whenever a scene handle is added to an entity consult the fab manager
/// and add a postfab if found. Postfabs are 'read-only' and can probably be
/// replaced with a reference/HashMap lookup so we don't have to worry about the performance of
/// a copy.
pub fn add_postfabs_to_spawned_scene(
    spawned_scenes: Query<(Entity, &SceneRoot), Added<SceneRoot>>,
    fab_manager: Res<FabManager>,
    mut cmds: Commands,
) {
    for (entity, spawned_scene) in spawned_scenes.iter() {
        let Some(postfab) = fab_manager.postfabs.get(&**spawned_scene) else {
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
        Query<(Entity, &PostFab, &SceneInstance, Option<&PostFabVariant>)>,
        Query<&Children>,
        Res<SceneSpawner>,
    )>::new(world);
    let (postfabs, children, scene_spawner) = system_state.get(world);

    let mut pipes_to_run = vec![];
    let mut root_entities = vec![];
    //For every entity with a postfab
    for (entity, postfab, instance, variant) in postfabs.iter() {
        if !scene_spawner.instance_is_ready(**instance) {
            continue;
        }

        //TODO: Figure out a way to not clone here >:(
        root_entities.push(entity);
        let pipe_iterator = match variant {
            Some(v) => Box::new(postfab.pipes.iter().chain(&v.variance))
                as Box<dyn Iterator<Item = &PostfabPipe>>,
            None => Box::new(postfab.pipes.iter()),
        };

        //Iterate over all of a postfabs pipe
        for pipe in pipe_iterator {
            let applicable_ents = match pipe.root_only {
                true => vec![entity],
                false => std::iter::once(entity)
                    .chain(children.iter_descendants(entity))
                    .collect(),
            };

            //Attempt to apply to the parent, then any children
            'child: for applicable_entity in applicable_ents {
                let Ok(ent) = world.get_entity(applicable_entity) else {
                    warn!("Could not get entity for postfab, aborting postfab");
                    continue;
                };

                //Check if enity has required Name
                match ent.get::<Name>() {
                    Some(n) => {
                        if !pipe.name_criteria.iter().all(|criteria| criteria.eval(n)) {
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
                pipes_to_run.push((pipe.executor.clone(), applicable_entity));
            }
        }
    }

    //Remove the postfab for the parent so it's not processed again
    for ent in root_entities {
        world.entity_mut(ent).remove::<PostFab>();
    }

    // Run the system with the entity as the input
    for (executor, ent) in pipes_to_run {
        match executor {
            RunType::System(system) => {
                if let Err(e) = world.run_system_with_input(system, ent) {
                    error!("Error running system for postfab pipe!\n {}", e);
                }
            }
            RunType::Command(cmd) => {
                cmd.dyn_add(&mut world.commands());
            }
            RunType::Entity(entcmd) => {
                let mut world_cmds = world.commands();
                let Some(mut entcmds) = world_cmds.get_entity(ent) else {
                    error!("Could not get entity for entity command postfab");
                    continue;
                };

                entcmd.dyn_add(&mut entcmds);
            }
        }
    }
    world.flush();
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

impl From<Vec<PostfabPipe>> for PostFabVariant {
    fn from(value: Vec<PostfabPipe>) -> Self {
        PostFabVariant { variance: value }
    }
}

#[derive(Clone, Component)]
pub struct PostFabVariant {
    pub variance: Vec<PostfabPipe>,
}

#[derive(Clone)]
pub enum RunType {
    System(SystemId<In<Entity>>),
    Entity(Box<dyn DynEntityCommand>),
    Command(Box<dyn DynCommand>),
}

/// An individual element of a postfab. Postfabs contain an ordered collection of pipes that run
/// in order. The pipe has various filtering functions etc. to make this easier. These filters could/should
/// be copied over to the prefab behavior as well
#[derive(Clone)]
pub struct PostfabPipe {
    pub executor: RunType,
    /// Only apply pipe to entities with the following components
    pub with_components: Vec<TypeId>,
    /// Only apply pipe to entities without the following components
    pub without_components: Vec<TypeId>,
    /// Only apply pipe to entities matching one of the  name criteria
    pub name_criteria: Vec<NameCriteria>,
    /// Only apply pipe to the scene root entity
    pub root_only: bool,
}

impl PostfabPipe {
    /// Run the system if an entity matches these criteria
    pub fn system(system: SystemId<In<Entity>, ()>) -> Self {
        Self {
            executor: RunType::System(system),
            with_components: vec![],
            without_components: vec![],
            name_criteria: vec![],
            root_only: false,
        }
    }

    /// Apply a command if it matches these criteria
    pub fn cmd(cmd: impl DynCommand) -> Self {
        Self {
            executor: RunType::Command(cmd.dyn_clone()),
            with_components: vec![],
            without_components: vec![],
            name_criteria: vec![],
            root_only: false,
        }
    }

    /// Apply an EntityCommand if it matches these criteria
    pub fn entity(cmd: impl DynEntityCommand) -> Self {
        Self {
            executor: RunType::Entity(cmd.dyn_clone()),
            with_components: vec![],
            without_components: vec![],
            name_criteria: vec![],
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
        self.name_criteria.push(NameCriteria::Equals(name.into()));
        self
    }

    /// Apply only to entities in the scene/root with a name containing the input
    pub fn name_contains(mut self, name: impl Into<String>) -> Self {
        self.name_criteria.push(NameCriteria::Contains(name.into()));
        self
    }

    /// Apply only to entities in the scene/root with a name containing one of the inputs
    pub fn name_contains_any(mut self, names: Vec<impl Into<String>>) -> Self {
        self.name_criteria.push(NameCriteria::Any(
            names
                .into_iter()
                .map(|v| NameCriteria::Contains(v.into()))
                .collect(),
        ));
        self
    }

    /// Apply only to entities in the scene/root with a name starting with the input
    pub fn name_starts_with(mut self, name: impl Into<String>) -> Self {
        self.name_criteria
            .push(NameCriteria::StartsWith(name.into()));
        self
    }

    pub fn name_starts_with_any(mut self, names: Vec<impl Into<String>>) -> Self {
        self.name_criteria.push(NameCriteria::Any(
            names
                .into_iter()
                .map(|v| NameCriteria::StartsWith(v.into()))
                .collect(),
        ));
        self
    }

    /// Apply only to entities in the scene/root with a name ending with the input
    pub fn name_ends_with(mut self, name: impl Into<String>) -> Self {
        self.name_criteria.push(NameCriteria::EndsWith(name.into()));
        self
    }

    pub fn name_ends_with_any(mut self, names: Vec<impl Into<String>>) -> Self {
        self.name_criteria.push(NameCriteria::Any(
            names
                .into_iter()
                .map(|v| NameCriteria::EndsWith(v.into()))
                .collect(),
        ));
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
    Any(Vec<NameCriteria>),
    Equals(String),
    Contains(String),
    StartsWith(String),
    EndsWith(String),
}

impl NameCriteria {
    pub fn eval(&self, name: &Name) -> bool {
        match self {
            NameCriteria::Any(criteria) => criteria.iter().any(|c| c.eval(name)),
            NameCriteria::Equals(c) => c == &name.to_string(),
            NameCriteria::Contains(c) => name.to_string().contains(c.as_str()),
            NameCriteria::StartsWith(c) => name.starts_with(c.as_str()),
            NameCriteria::EndsWith(c) => name.ends_with(c.as_str()),
        }
    }
}
