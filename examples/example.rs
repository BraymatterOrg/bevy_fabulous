use std::f32::consts::PI;

use bevy::{core_pipeline::bloom::BloomSettings, prelude::*};
use bevy_fabulous::{prefab::{Prefab, PrefabPipe}, FabManager, FabulousPlugin};

fn main() {
    let mut app = App::new();
    app.register_type::<Rotate>();
    app.register_type::<Bob>();

    app.add_plugins(DefaultPlugins);
    app.add_plugins(FabulousPlugin);
    
    app.insert_state(GameState::Loading);

    //Load minion asset, and wait until it's loaded
    app.add_systems(Startup, load_minion_asset);
    app.add_systems(Update, poll_loaded.run_if(in_state(GameState::Loading)));

    //Only run setup when minion is loaded
    app.add_systems(OnEnter(GameState::Loaded), setup_scene);

    //Spin me _right_ round
    app.add_systems(Update, (rotate_over_time, bob));
    app.run();
}

fn setup_scene(mut cmds: Commands, ex: Res<ExampleResource>) {
    //Spawn Camera
    cmds.spawn((Camera3dBundle {
        camera: Camera {
            hdr: true,
            clear_color: ClearColorConfig::Custom(Color::BLACK.lighter(0.5)),
            ..default()
        },
        transform: Transform::from_translation(Vec3::new(10.0, 10.0, 10.0))
            .looking_at(Vec3::ZERO, Dir3::Y),
        ..default()
    }, BloomSettings::OLD_SCHOOL));

    info!("Spawning Minion");
    //Spawn Minion
    cmds.spawn(SceneBundle {
        scene: ex.asset_scene.clone(),
        ..default()
    });

    //Shine a little light on me
    cmds.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            color: Color::LinearRgba(LinearRgba {
                red: 0.8,
                green: 0.8,
                blue: 0.8,
                alpha: 1.0,
            }),
            ..default()
        },
        ..default()
    });
}

fn load_minion_asset(
    asset_server: ResMut<AssetServer>,
    mut cmds: Commands,
    mut fabs: ResMut<FabManager>,
) {
    let scene_handle = asset_server.load("earthminion.glb#Scene0");

    cmds.insert_resource(ExampleResource {
        asset_scene: scene_handle.clone(),
    });

    fabs.register_prefab(
        Prefab::new("earthminion.glb#Scene0")
            .with_system(inner_gear_rotate)
            .with_pipe(HeadPipe {
                rotation_rate: PI / 10.0,
            }),
    );
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
    pub asset_scene: Handle<Scene>,
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
        tsf.translation.y = bobber.anchor + (time.elapsed_seconds() * bobber.frequency).sin() * (bobber.amplitude );
    }
}


