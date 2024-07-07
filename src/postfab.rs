use std::marker::PhantomData;

use bevy::{ecs::{component::StorageType, world::{Command, DeferredWorld}}, prelude::*};


/// Postfabs are used to modify a scene every time it's spawned
/// You may use these to pass in contextual information information at the time
/// of spawning such as changing the material color based on health / faction etc.
/// These run every time you spawn the PostFab
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
                pipe.apply(&mut world, targeted_entity)
            }

            world
                .commands()
                .add(RemoveComponent::<PostFab>::new(targeted_entity));
        });
    }
}

pub trait PostfabPipe {
    fn apply(&mut self, world: &mut DeferredWorld, targeted_entity: Entity);
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