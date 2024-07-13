use std::f32::consts::PI;

use bevy::{color::palettes, core_pipeline::bloom::BloomSettings, prelude::*};
use bevy_fabulous::{
    materials::{FabulousMaterialsPlugin, NamedMaterialIndex},
    postfab::{PostFab, PostfabPipe},
    prefab::{Prefab, PrefabPipe},
    FabManager, FabTarget, FabulousPlugin, GltfScene, SpawnGltfCmdExt,
};

fn main() {
    let mut app = App::new();
    app.register_type::<Rotate>();
    app.register_type::<Bob>();

    app.add_plugins(DefaultPlugins);
    app.add_plugins((
        FabulousPlugin,
        FabulousMaterialsPlugin::<StandardMaterial>::default(),
    ));

    app.insert_state(GameState::Loading);

    //Load minion asset, and wait until it's loaded
    app.add_systems(Startup, load_minion_asset);
    app.add_systems(Update, poll_loaded.run_if(in_state(GameState::Loading)));

    //Only run setup when minion is loaded
    app.add_systems(OnEnter(GameState::Loaded), setup_scene);

    //Spin me _right_ round
    app.add_systems(Update, (rotate_over_time, bob, scale));
    app.run();
}

fn setup_scene(mut cmds: Commands, ex: Res<ExampleResource>) {
    //Spawn Camera
    cmds.spawn((
        Camera3dBundle {
            camera: Camera {
                hdr: true,
                clear_color: ClearColorConfig::Custom(Color::BLACK.lighter(0.03)),
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(10.0, 10.0, 10.0))
                .looking_at(Vec3::ZERO, Dir3::Y),
            ..default()
        },
        BloomSettings::OLD_SCHOOL,
    ));

    info!("Spawning Minion");

    // Spawn Minion
    cmds.spawn_gltf(GltfScene::new(ex.asset_scene.clone()).with_bundle(Name::new("Minion")));

    // Shine a little light on me
    cmds.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            color: Color::LinearRgba(LinearRgba {
                red: 0.8,
                green: 0.8,
                blue: 0.8,
                alpha: 1.0,
            }),
            illuminance: 600.0,
            ..default()
        },
        ..default()
    });
}

fn load_minion_asset(
    asset_server: ResMut<AssetServer>,
    mut cmds: Commands,
    mut fabs: ResMut<FabManager>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut mat_index: ResMut<NamedMaterialIndex<StandardMaterial, StandardMaterial>>,
) {
    let gltf_handle = asset_server.load("earthminion.glb");

    cmds.insert_resource(ExampleResource {
        asset_scene: gltf_handle.clone(),
    });

    //Create and register new material to be swapped out
    let earth_mana = StandardMaterial {
        emissive: (palettes::css::LIMEGREEN * 2.0).into(),
        ..default()
    };

    mat_index.register_main_mat("EarthMana", mats.add(earth_mana));

    fabs.register_prefab(
        Prefab::new(FabTarget::Gltf(gltf_handle.clone()))
            .with_system(inner_gear_rotate)
            .with_pipe(RotateHeadPipe {
                rotation_rate: PI / 10.0,
            }),
    );

    fabs.register_postfab(PostFab {
        scene: FabTarget::Gltf(gltf_handle),
        pipes: vec![
            PostfabPipe::new(cmds.register_one_shot_system(add_scalar_to_orbiters))
                .name_contains("Orbiter")
                .root_only(),
        ],
    })
}

fn poll_loaded(
    asset_server: Res<AssetServer>,
    ex: Res<ExampleResource>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if asset_server.is_loaded_with_dependencies(ex.asset_scene.id()) {
        next_state.set(GameState::Loaded);
    }
}

#[derive(Resource)]
pub struct ExampleResource {
    pub asset_scene: Handle<Gltf>,
}

#[derive(States, Debug, Clone, Eq, PartialEq, PartialOrd, Hash)]
pub enum GameState {
    Loading,
    Loaded,
}
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

//Define a prefab pipe as a struct
pub struct RotateHeadPipe {
    pub rotation_rate: f32,
}

impl PrefabPipe for RotateHeadPipe {
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

fn add_scalar_to_orbiters(target: In<Entity>, mut cmds: Commands, names: Query<&Name>) {
    info!("Adding Scalar: {}", names.get(target.0).unwrap());
    cmds.entity(target.0).insert(ScaleOverTime {
        factor: Vec3::splat(1.2),
        frequency: 1.0,
        base_scale: Vec3::splat(1.0),
    });
}

//Scene Systems!

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Rotate {
    rotation_rate: f32,
}

pub fn rotate_over_time(mut rotators: Query<(&mut Transform, &Rotate)>, time: Res<Time>) {
    for (mut tsf, rotator) in rotators.iter_mut() {
        tsf.rotate_y(rotator.rotation_rate * time.delta_seconds());
    }
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Bob {
    amplitude: f32,
    frequency: f32,
    anchor: f32,
}

pub fn bob(mut bobbers: Query<(&mut Transform, &Bob)>, time: Res<Time>) {
    for (mut tsf, bobber) in bobbers.iter_mut() {
        tsf.translation.y =
            bobber.anchor + (time.elapsed_seconds() * bobber.frequency).sin() * (bobber.amplitude);
    }
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct ScaleOverTime {
    pub factor: Vec3,
    pub frequency: f32,
    pub base_scale: Vec3,
}

fn scale(mut scaler: Query<(&mut Transform, &ScaleOverTime)>, time: Res<Time>) {
    for (mut tsf, scaler) in scaler.iter_mut() {
        tsf.scale = scaler.base_scale
            + (time.elapsed_seconds() * scaler.frequency).sin().abs()
                * (scaler.factor - scaler.base_scale);
    }
}
