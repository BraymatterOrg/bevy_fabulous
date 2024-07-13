# Bevy Fabulous

> I want to tell you a story. You order some furniture online. It arrives in a box and the delivery man rings the doorbell.
> Before you're able to get to the door a goblin opens the box, assembles the furniture and puts it back in the box. Fabulous.

https://github.com/user-attachments/assets/592b0c02-baa8-4e93-a0c7-7d6627ef6ade

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
    pub target: FabTarget,

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
    Prefab::new(FabTarget::Gltf(gltf_handle.clone()))
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

Postfabs are run every time a specific Scene is spawned. They run on entities _after_ they are spawned, and do not modify
the source Scene.

- They can be used to transform a spawned scene based on contextual information in an ergonomic way.
- They can be used to replace materials / assets from the Scene with the 'Hydrated' assets from the main Bevy world. E.g. `StandardMaterial`
- If possible, use a Prefab as the one-time cost is preferable to running logic/queries every time the Scene is spawned

Postfabs can be registered with a `FabTarget` which provides for the user to use either a `Handle<Scene>` directly, or a `Handle<Gltf>`, which will register the postfab
with the first scene in the gltf.

```rs
#[derive(Clone, Component)]
pub struct PostFab {
    pub scene: FabTarget,
    pub pipes: Vec<PostfabPipe>,
}
```

Like `Prefabs,`, `Postfabs` are composed of a FabTarget and a series of pipes applied in order. The `PostfabPipe` has som more advanced filtering options to specify
whether a pipe should run on a given entity:

```rs
#[derive(Clone)]
pub struct PostfabPipe {
    pub system: SystemId<Entity, ()>,
    pub with_components: Vec<TypeId>,
    pub without_components: Vec<TypeId>,
    pub with_name: Option<NameCriteria>,
}

/// Name component criteria for determining whether a pipe should run on a given entity
#[derive(Clone)]
pub enum NameCriteria {
    Equals(String),
    Contains(String),
    StartsWith(String),
    EndsWith(String),
}
```

## Material Overrides

Material overrides are used to automatically replace material handles on entity with another. This is useful for replacing the standard mat loaded as part of
a scene/gltf with something custom. The name is fetched from the GLTF asset's `NamedMaterials` map. At time of writing the user needs to load the gltf asset directly
so it's alive and available at the time the `FabulousMaterialsPlugin` asset watcher runs. This means Scenes loaded with "myAsset.gltf#Scene0" or
`GltfAssetLabel::Scene(0)` may not work as intended.

```rs

//Create and register new material to be swapped out
let earth_mana = StandardMaterial {
    emissive: (palettes::css::LIMEGREEN * 2.0).into(),
    ..default()
};

mat_index.register_main_mat("EarthMana", mats.add(earth_mana));
```

### Spawning Gltf Scenes
Because the Named Material requires that a GLTF scene be available at the time the scene asset is loaded, it's best
to load the Gltf instead of the Scene with in it directly. To make this a easier to deal with this crate provides the `SpawnGltfScene` command and some helpers. You can spawn a specific scene from a GLTF like so:

```rs
    // Spawn Minion at Default location
    cmds.spawn_gltf(GltfScene::new(ex.asset_scene.clone()).with_bundle(Name::new("Minion")));
```

GltfScene also provides `at_location(Transform)`, `with_scene(usize)`, and `build()` for specifying the transform, which scene in the gltf, and spawning a scene without any additional components on the scene root
