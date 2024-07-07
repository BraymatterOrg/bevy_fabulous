# Bevy Fabulous

> I want to tell you a story. You order some furniture online. It arrives in a box and the delivery man rings the doorbell. 
Before you're able to get to the door a goblin opens the box, assembles the furniture and puts it back in the box. Fabulous. 

## Overview

Bevy fabulous aims to provide a framework for encapsulating and coupling a loaded GLTF to it's gameplay components without
using heavy tooling, or opinionated plugins. 

Bevy Fabulous provides to mechanisms to enrich a GLTF loaded Scene: **Prefabs** and **PostFabs**

## Prefabs

Prefabs modify the loaded scene world directly, applying gameplay components to entities in the Scene world directly. This only
needs to run once as the components are then part of the Scene, so any subsequent spawns of `SceneBundle` will have the components. 

Prefabs contain a path to the asset, which is used as the key to a `HashMap<String, Prefab>` in the `FabManager` resource. 

Prefabs are a wrapper around a Vec of PrefabPipe trait objects, and run in the order they are inserted:

```rs
/// Applies ScenePipes to the loaded scene `World`
pub struct Prefab {
    /// The path to the asset on the filesystem
    pub path: String,

    /// Pipes to run on load
    pub pipeline: Vec<Box<dyn PrefabPipe + Send + Sync>>,
}
```

`PrefabPipe` can end up looking a lot like a Commands:

```rs
///Used to transform a scene, but avoid Transform as a term - it's already overloaded
pub trait PrefabPipe {
    // Applies the pipe to the entity
    fn apply(&mut self, world: &mut World);
}

//Define a prefab pipe as a struct
pub struct HeadPipe {
    pub rotation_rate: f32,
}

impl PrefabPipe for HeadPipe {
    fn apply(&mut self, world: &mut World) {
        info!("Running Head Rotate Pipe");

        //Iterate over the entities in the world and find the homie with the a head on him
        let mut q = world.query::<(Entity, &Name)>();
        let q = q.iter(world);

        let mut ent: Option<Entity> = None;
        for (entity, name) in q {
            if *name.to_string() == "MinionHead".to_string() {
                ent = Some(entity);
                break;
            }
        }

        if let Some(entity) = ent {
            info!("Found Thing and attaching to stuff");
            world.entity_mut(entity).insert((
                Rotate {
                    rotation_rate: self.rotation_rate,
                },
                Bob {
                    amplitude: 0.4,
                    frequency: 0.8,
                    anchor: 2.0,
                },
            ));
        }

        world.flush();
    }
}
```

_`PrefabPipe` also has a blanket implementation for BoxedSystem! Allowing you to use any System as a PrefabPipe:_

```rs
//In Some Startup/Loaded System
//--- 
//Register to the `FabManager`
fabs.register_prefab(
        Prefab::new("earthminion.glb#Scene0")
            .with_system(inner_gear_rotate)
    );
//---

//Define a prefab pipe as a system
fn inner_gear_rotate(entities: Query<(Entity, &Name)>, mut cmds: Commands) {
    info!("Inner Gear Rotate");

    let mut gear_ent = None;
    let mut orbiters = vec![];

    for (ent, name) in entities.iter() {
        let name_str = name.as_str();
        if name_str == "Gear" {
            gear_ent = Some(ent);
        } else if name_str.contains("Orbiter") {
            orbiters.push(ent);
        }
    }

    let Some(entity) = gear_ent else {
        return;
    };

    cmds.entity(entity).insert(Rotate {
        rotation_rate: -0.5,
    });

    for orbiter in orbiters {
        cmds.entity(orbiter).insert(Rotate { rotation_rate: 0.5 });
    }
}
```

## Postfabs

Postfabs are run everytime a specific Scene is spawned. They run on entities _after_ they are spawned, and do not modify
the source Scene. 

- They can be used to transform a spawned scene based on contextual information in an ergonomic way.
- They can be used to replace materials / assets from the Scene with the 'Hydrated' assets from the main Bevy world. E.g. `StandardMaterial`
- If possible, use a Prefab as the one-time cost is preferable to running logic/queries every time the Scene is spawned

Postfabs work by using a `Component::on_add` hook for `PostFab`. This means in addition to a Scene/Bundle you need to attach a PostFab
to your spawned `Entity`. 

`Postfabs` are stored on the `FabManager` as a `HashMap<Handle<Scene>, PostFab>` and can be fetched for a given `Scene`

Like `Prefabs`, `PostFabs` are comprised of a Vec of `PostFabPipe` that runs in order of insertion:

```rs
pub struct PostFab {
    pub scene: Handle<Scene>,
    pub pipes: Vec<Box<dyn PostfabPipe + Send + Sync>>,
}
```

