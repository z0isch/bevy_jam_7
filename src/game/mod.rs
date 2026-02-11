use bevy::{prelude::*, window::PrimaryWindow};
use bevy_enhanced_input::prelude::*;
use bevy_rand::{global::GlobalRng, prelude::WyRand};
use bevy_rapier3d::prelude::*;
use rand::Rng;

use crate::IsometricCamera;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(EnhancedInputPlugin);
    app.add_input_context::<Player>();
    app.add_observer(apply_movement);
    app.add_systems(FixedUpdate, enemy_chase_player);
    app.add_systems(Update, (aim_spotlight, camera_follow));
}

#[derive(Component)]
pub struct Player;

#[derive(Component)]
struct PlayerSpotlight;

#[derive(InputAction)]
#[action_output(Vec2)]
struct Movement;

#[derive(Component)]
struct Enemy;

#[derive(Component)]
struct SpeedFactor(f32);

pub fn spawn_game(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(1000.0, 1000.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
        Collider::cuboid(1000.0, 0.1, 1000.0),
        Transform::from_xyz(-100.0, 0., -100.0),
    ));

    commands.spawn((
        Name::new("Player"),
        Player,
        actions!(Player[
            (
                Action::<Movement>::new(),
                DeadZone::default(),
                SmoothNudge::default(),
                Bindings::spawn((
                    Cardinal::wasd_keys(),
                    Axial::left_stick(),
                )),
            ),
        ]),
        RigidBody::KinematicPositionBased,
        Collider::cuboid(0.5, 0.5, 0.5),
        Transform::from_xyz(0.0, 0.5, 0.0),
        KinematicCharacterController {
            apply_impulse_to_dynamic_bodies: true,
            ..KinematicCharacterController::default()
        },
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::srgb(0.0, 0.0, 0.0))),
    ));

    for i in 0..10 {
        let x = rng.random_range(-50.0..50.0);
        let z = rng.random_range(-50.0..50.0);
        let speed_factor = rng.random_range(2.0..4.0);
        commands.spawn((
            Enemy,
            Name::new(format!("Enemy {}", i)),
            Mesh3d(meshes.add(Cuboid::default())),
            MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
            RigidBody::Dynamic,
            Collider::cuboid(0.5, 0.5, 0.5),
            Transform::from_xyz(x, 1., z),
            Velocity::default(),
            LockedAxes::TRANSLATION_LOCKED | LockedAxes::ROTATION_LOCKED_X,
            Ccd::enabled(),
            SpeedFactor(speed_factor),
        ));
    }

    // lights
    commands.spawn((
        PlayerSpotlight,
        SpotLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
}

fn apply_movement(
    movement: On<Fire<Movement>>,
    mut controller: Single<&mut KinematicCharacterController>,
    time: Res<Time>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform)>,
    player: Single<&Transform, With<Player>>,
    spotlight: Single<&mut Transform, (With<PlayerSpotlight>, Without<Player>)>,
) {
    let speed = 10.0;
    let input = movement.value;

    let forward = Vec3::new(-1.0, 0.0, -1.0).normalize();
    let right = Vec3::new(1.0, 0.0, -1.0).normalize();

    let direction = forward * input.y + right * input.x;

    controller.translation = Some(direction * speed * time.delta_secs());
    aim_spotlight(window, camera, player, spotlight);
}

fn aim_spotlight(
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform)>,
    player: Single<&Transform, With<Player>>,
    mut spotlight: Single<&mut Transform, (With<PlayerSpotlight>, Without<Player>)>,
) {
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    let (camera, camera_transform) = *camera;
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos) else {
        return;
    };

    // Intersect the camera ray with the ground plane (y = -2).
    let ground_y = -2.0;
    let denom = ray.direction.y;
    if denom.abs() < 1e-6 {
        return;
    }
    let t = (ground_y - ray.origin.y) / denom;
    if t < 0.0 {
        return;
    }
    let mouse_ground = ray.origin + *ray.direction * t;

    let player_pos = player.translation;
    spotlight.translation = player.translation;

    // Aim the spotlight in the XZ direction toward the mouse, keeping it level.
    let dir = (mouse_ground - player_pos) * Vec3::new(1.0, 0.0, 1.0);
    if dir.length_squared() > 1e-6 {
        let target = player_pos + dir.normalize();
        spotlight.look_at(target, Vec3::Y);
    }
}

fn enemy_chase_player(
    player: Single<&Transform, (With<Player>, Without<Enemy>)>,
    mut enemies: Query<(&Transform, &mut Velocity, &SpeedFactor), With<Enemy>>,
) {
    let player_pos = player.translation;

    for (enemy_transform, mut velocity, speed_factor) in &mut enemies {
        let direction = (player_pos - enemy_transform.translation) * Vec3::new(1.0, 0.0, 1.0);
        if direction.length_squared() > 0.01 {
            let dir = direction.normalize();
            velocity.linvel = dir * speed_factor.0;
        }
    }
}

fn camera_follow(
    player: Single<&Transform, With<Player>>,
    mut camera: Single<
        (&mut Transform, &IsometricCamera),
        (Without<Player>, Without<PlayerSpotlight>),
    >,
    time: Res<Time>,
) {
    let (ref mut cam_transform, iso_cam) = *camera;
    let target_pos = player.translation + iso_cam.offset;

    // Smooth follow — adjust the speed factor to taste (higher = snappier)
    let smoothness = 8.0;
    cam_transform.translation = cam_transform
        .translation
        .lerp(target_pos, smoothness * time.delta_secs());
}
