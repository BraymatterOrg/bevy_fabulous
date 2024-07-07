use bevy::{ecs::system::BoxedSystem, prelude::*};
use bevy_prefabulous::{pipe, FabManager, FabulousPlugin, Prefab, PrefabPipe};

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins);
    app.add_plugins(FabulousPlugin);

    app.add_systems(Startup, load_minion_asset);

    app.run();
}

fn load_minion_asset(
    asset_server: ResMut<AssetServer>,
    mut cmds: Commands,
    mut fabs: ResMut<FabManager>,
) {
    let gltf_handle = asset_server.load("earthminion.glb");

    cmds.insert_resource(ExampleResource {
        asset_scene: gltf_handle,
    });

    fabs.register_loaded_prefab(Prefab {
        path: "earthminion.glb".to_string(),
        node_pipe: vec![
            pipe(get_thing),
            Box::new(OuterGearRotate {
                rotation_rate: 6.28,
            }),
        ],
    });
}

fn get_thing() -> BoxedSystem {
    Box::new(IntoSystem::into_system(inner_gear_rotate)) as BoxedSystem
}

#[derive(Resource)]
pub struct ExampleResource {
    pub asset_scene: Handle<Gltf>,
}

//Define a prefab pipe as a system
fn inner_gear_rotate(entities: Query<(Entity, &Name)>) {
    info!("Inner Gear Rotate");

    for (ent, name) in entities.iter() {
        info!("Found entity: {}", name);
    }
}

//Define a prefab pipe as a struct
pub struct OuterGearRotate {
    pub rotation_rate: f32,
}

impl PrefabPipe for OuterGearRotate {
    fn apply(&mut self, world: &mut World) {
        info!("I AM A PIPE I AM A PIPE");
    }
}
