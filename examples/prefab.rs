use bevy::prelude::*;
use bevy_prefabulous::{FabManager, FabulousPlugin, Prefab, PrefabPipe};

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins);
    app.add_plugins(FabulousPlugin);

    app.add_systems(Startup, (load_minion_asset, setup_scene).chain());
    
    app.run();
}

fn setup_scene(){
    
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

    fabs.register_prefab(
        Prefab::new("earthminion.glb")
            .with_system(inner_gear_rotate)
            .with_pipe(OuterGearRotate {
                rotation_rate: 6.28,
            }),
    )
}

//TODO: Impl trait for fn that returns this closure?
// fn gen_gear_rotate_sys() -> BoxedSystem {
//     Box::new(IntoSystem::into_system(inner_gear_rotate)) as BoxedSystem
// }

#[derive(Resource)]
pub struct ExampleResource {
    pub asset_scene: Handle<Gltf>,
}

//Define a prefab pipe as a system
fn inner_gear_rotate(entities: Query<&Name>) {
    info!("Inner Gear Rotate");

    for name in entities.iter() {
        info!("Found entity: {}", name);
    }
}

//Define a prefab pipe as a struct
pub struct OuterGearRotate {
    pub rotation_rate: f32,
}

impl PrefabPipe for OuterGearRotate {
    fn apply(&mut self, _world: &mut World) {
        info!("I AM A PIPE I AM A PIPE");
    }
}
