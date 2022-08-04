// This file is heavily inspired by the Bevy scene_viewer.rs
// found in the repository's tools folder.
//! A simple FBX scene viewer made with Bevy.
//!
//! Just run `cargo run --release --example scene_viewer /path/to/model.fbx#Scene`,
//! (note: the #Scene is added if not already present)
//! replacing the path as appropriate.
//! With no arguments it will load the default cube.

use bevy::{
    asset::AssetServerSettings,
    input::mouse::MouseMotion,
    log::{Level, LogSettings},
    math::Vec3A,
    prelude::*,
    render::primitives::{Aabb, Sphere},
    window::close_on_esc,
};
use bevy_fbx::FbxPlugin;
use bevy_inspector_egui::WorldInspectorPlugin;

use std::f32::consts::TAU;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
struct CameraControllerCheckSystem;

fn main() {
    println!(
        "
Controls:
    MOUSE       - Move camera orientation
    LClick/M    - Enable mouse movement
    WSAD        - forward/back/strafe left/right
    LShift      - 'run'
    E           - up
    Q           - down
    L           - animate light direction
    U           - toggle shadows
    C           - cycle through cameras
    5/6         - decrease/increase shadow projection width
    7/8         - decrease/increase shadow projection height
    9/0         - decrease/increase shadow projection near/far

"
    );
    let mut app = App::new();
    app.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 1.0 / 5.0f32,
    })
    .insert_resource(LogSettings {
        level: Level::WARN,
        filter: "bevy_fbx=info".to_owned(),
    })
    .insert_resource(AssetServerSettings {
        asset_folder: std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string()),
        watch_for_changes: true,
    })
    .insert_resource(WindowDescriptor {
        title: "bevy scene viewer".to_string(),
        ..default()
    })
    .add_plugins(DefaultPlugins)
    .add_plugin(WorldInspectorPlugin::new())
    .add_plugin(FbxPlugin)
    .add_startup_system(setup)
    .add_system(update_lights)
    .add_system(camera_controller)
    .add_system(close_on_esc);

    app.run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut scene_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "assets/cube.fbx".to_owned());
    if !scene_path.ends_with("#Scene") {
        scene_path += "#Scene";
    }
    info!("Loading {}", scene_path);
    commands
        .spawn_bundle(Camera3dBundle {
            transform: Transform::from_xyz(10.0, 4.4, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        })
        .insert(CameraController::default());

    let sphere = Sphere {
        center: Vec3A::ONE * 3.0,
        radius: 100.0,
    };
    let aabb = Aabb::from(sphere);
    let min = aabb.min();
    let max = aabb.max();

    info!("Spawning a directional light");
    commands.spawn_bundle(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 20000.0,
            shadow_projection: OrthographicProjection {
                left: min.x,
                right: max.x,
                bottom: min.y,
                top: max.y,
                near: min.z,
                far: max.z,
                ..default()
            },
            shadows_enabled: false,
            ..default()
        },
        ..default()
    });
    commands
        .spawn_bundle(SceneBundle {
            scene: asset_server.load(&scene_path),
            ..default()
        })
        .insert(Name::new(scene_path));
}

const SCALE_STEP: f32 = 0.1;

fn update_lights(
    key_input: Res<Input<KeyCode>>,
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut DirectionalLight)>,
    mut animate_directional_light: Local<bool>,
) {
    let mut projection_adjustment = Vec3::ONE;
    if key_input.just_pressed(KeyCode::Key5) {
        projection_adjustment.x -= SCALE_STEP;
    } else if key_input.just_pressed(KeyCode::Key6) {
        projection_adjustment.x += SCALE_STEP;
    } else if key_input.just_pressed(KeyCode::Key7) {
        projection_adjustment.y -= SCALE_STEP;
    } else if key_input.just_pressed(KeyCode::Key8) {
        projection_adjustment.y += SCALE_STEP;
    } else if key_input.just_pressed(KeyCode::Key9) {
        projection_adjustment.z -= SCALE_STEP;
    } else if key_input.just_pressed(KeyCode::Key0) {
        projection_adjustment.z += SCALE_STEP;
    }
    for (_, mut light) in query.iter_mut() {
        light.shadow_projection.left *= projection_adjustment.x;
        light.shadow_projection.right *= projection_adjustment.x;
        light.shadow_projection.bottom *= projection_adjustment.y;
        light.shadow_projection.top *= projection_adjustment.y;
        light.shadow_projection.near *= projection_adjustment.z;
        light.shadow_projection.far *= projection_adjustment.z;
        if key_input.just_pressed(KeyCode::U) {
            light.shadows_enabled = !light.shadows_enabled;
        }
    }

    if key_input.just_pressed(KeyCode::L) {
        *animate_directional_light = !*animate_directional_light;
    }
    if *animate_directional_light {
        for (mut transform, _) in query.iter_mut() {
            transform.rotation = Quat::from_euler(
                EulerRot::ZYX,
                0.0,
                time.seconds_since_startup() as f32 * TAU / 30.0,
                -TAU / 8.,
            );
        }
    }
}

#[derive(Component)]
struct CameraController {
    pub enabled: bool,
    pub initialized: bool,
    pub sensitivity: f32,
    pub key_forward: KeyCode,
    pub key_back: KeyCode,
    pub key_left: KeyCode,
    pub key_right: KeyCode,
    pub key_up: KeyCode,
    pub key_down: KeyCode,
    pub key_run: KeyCode,
    pub mouse_key_enable_mouse: MouseButton,
    pub keyboard_key_enable_mouse: KeyCode,
    pub walk_speed: f32,
    pub run_speed: f32,
    pub friction: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub velocity: Vec3,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            enabled: true,
            initialized: false,
            sensitivity: 0.5,
            key_forward: KeyCode::W,
            key_back: KeyCode::S,
            key_left: KeyCode::A,
            key_right: KeyCode::D,
            key_up: KeyCode::E,
            key_down: KeyCode::Q,
            key_run: KeyCode::LShift,
            mouse_key_enable_mouse: MouseButton::Left,
            keyboard_key_enable_mouse: KeyCode::M,
            walk_speed: 5.0,
            run_speed: 15.0,
            friction: 0.5,
            pitch: 0.0,
            yaw: 0.0,
            velocity: Vec3::ZERO,
        }
    }
}

fn camera_controller(
    time: Res<Time>,
    mut mouse_events: EventReader<MouseMotion>,
    mouse_button_input: Res<Input<MouseButton>>,
    key_input: Res<Input<KeyCode>>,
    mut move_toggled: Local<bool>,
    mut query: Query<(&mut Transform, &mut CameraController), With<Camera>>,
) {
    let dt = time.delta_seconds();

    if let Ok((mut transform, mut options)) = query.get_single_mut() {
        if !options.initialized {
            let (yaw, pitch, _roll) = transform.rotation.to_euler(EulerRot::YXZ);
            options.yaw = yaw;
            options.pitch = pitch;
            options.initialized = true;
        }
        if !options.enabled {
            return;
        }

        // Handle key input
        let mut axis_input = Vec3::ZERO;
        if key_input.pressed(options.key_forward) {
            axis_input.z += 1.0;
        }
        if key_input.pressed(options.key_back) {
            axis_input.z -= 1.0;
        }
        if key_input.pressed(options.key_right) {
            axis_input.x += 1.0;
        }
        if key_input.pressed(options.key_left) {
            axis_input.x -= 1.0;
        }
        if key_input.pressed(options.key_up) {
            axis_input.y += 1.0;
        }
        if key_input.pressed(options.key_down) {
            axis_input.y -= 1.0;
        }
        if key_input.just_pressed(options.keyboard_key_enable_mouse) {
            *move_toggled = !*move_toggled;
        }

        // Apply movement update
        if axis_input != Vec3::ZERO {
            let max_speed = if key_input.pressed(options.key_run) {
                options.run_speed
            } else {
                options.walk_speed
            };
            options.velocity = axis_input.normalize() * max_speed;
        } else {
            let friction = options.friction.clamp(0.0, 1.0);
            options.velocity *= 1.0 - friction;
            if options.velocity.length_squared() < 1e-6 {
                options.velocity = Vec3::ZERO;
            }
        }
        let forward = transform.forward();
        let right = transform.right();
        transform.translation += options.velocity.x * dt * right
            + options.velocity.y * dt * Vec3::Y
            + options.velocity.z * dt * forward;

        // Handle mouse input
        let mut mouse_delta = Vec2::ZERO;
        if mouse_button_input.pressed(options.mouse_key_enable_mouse) || *move_toggled {
            for mouse_event in mouse_events.iter() {
                mouse_delta += mouse_event.delta;
            }
        }

        if mouse_delta != Vec2::ZERO {
            // Apply look update
            let (pitch, yaw) = (
                (options.pitch - mouse_delta.y * 0.5 * options.sensitivity * dt).clamp(
                    -0.99 * std::f32::consts::FRAC_PI_2,
                    0.99 * std::f32::consts::FRAC_PI_2,
                ),
                options.yaw - mouse_delta.x * options.sensitivity * dt,
            );
            transform.rotation = Quat::from_euler(EulerRot::ZYX, 0.0, yaw, pitch);
            options.pitch = pitch;
            options.yaw = yaw;
        }
    }
}
