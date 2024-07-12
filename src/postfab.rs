use std::any::TypeId;

use bevy::{
    ecs::system::{SystemId, SystemState},
    prelude::*,
    scene::SceneInstance,
};

use crate::{FabManager, PostFabTarget};

/// Whenever a scene handle is added to an entity consult the fab manager
/// and add a postfab if found
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
            //Attempt to apply to any children
            'child: for applicable_entity in
                std::iter::once(entity).chain(children.iter_descendants(entity))
            {
                let Some(ent) = world.get_entity(applicable_entity) else {
                    warn!("Could not get entity for postfab, aborting postfab");
                    continue;
                };
                
                //Check if enity has required Name
                if let Some(criteria) = &pipe.with_name {
                    match ent.get::<Name>() {
                        Some(n) => {
                            if !criteria.eval(n) {
                                continue 'child;
                            }
                        }
                        None => {
                            info!("Name not found");
                            continue 'child;
                        }
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

    for ent in root_entities {
        world.entity_mut(ent).remove::<PostFab>();
    }

    for (system, ent) in pipes_to_run {
        if let Err(e) = world.run_system_with_input(system, ent) {
            error!("Error running system for postfab pipe!\n {}", e);
        }
    }
}

/// Postfabs are used to modify a scene every time it's spawned
/// You may use these to pass in contextual information information at the time
/// of spawning such as changing the material color based on health / faction etc.
/// These run every time you spawn the PostFab
#[derive(Clone, Component)]
pub struct PostFab {
    pub scene: PostFabTarget,
    pub pipes: Vec<PostfabPipe>,
}

#[derive(Clone)]
pub struct PostfabPipe {
    pub system: SystemId<Entity, ()>,
    pub with_components: Vec<TypeId>,
    pub without_components: Vec<TypeId>,
    pub with_name: Option<NameCriteria>,
}

#[derive(Clone)]
pub enum NameCriteria {
    Equals(String),
    Contains(String),
    StartsWith(String),
    EndsWith(String),
}

impl NameCriteria {
    pub fn eval(&self, name: &Name) -> bool {
        match self {
            NameCriteria::Equals(c) => c == &name.to_string(),
            NameCriteria::Contains(c) => name.to_string().contains(c.as_str()),
            NameCriteria::StartsWith(c) => name.starts_with(c.as_str()),
            NameCriteria::EndsWith(c) => name.ends_with(c.as_str()),
        }
    }
}
