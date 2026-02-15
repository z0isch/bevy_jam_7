use std::collections::HashSet;

use bevy::{prelude::*, window::PrimaryWindow};
use bevy_enhanced_input::prelude::*;
use bevy_mesh::VertexAttributeValues;
use bevy_rand::{global::GlobalRng, prelude::WyRand};
use bevy_rapier3d::prelude::*;
use rand::Rng;

use crate::{
    IsometricCamera, PausableSystems,
    crt_postprocess::CrtSettings,
    game::{GameAssets, GameState, GameStateMachine, LIGHT_COLOR},
    screens::Screen,
};

pub const TORCH_COLOR: Color = Color::srgb(1.0, 90. / 255., 30. / 255.);
pub const MIRROR_COLOR: Color = Color::srgb(0.0, 200. / 255., 1.0);

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameStateMachine::Level), spawn_level);

    // Cursed controls
    app.init_resource::<CursedControls>();
    app.init_resource::<CursedAimState>();

    // Input observer
    app.add_plugins(EnhancedInputPlugin);
    app.add_input_context::<Player>();
    app.add_observer(apply_movement);

    // Gameplay systems
    app.add_systems(
        Update,
        (
            tick_player_time,
            (
                toggle_cursed_controls,
                enemy_chase_player,
                aim_spotlight,
                update_reflected_spotlight, // mirror bounce (A + C)
                check_spotlight,
                on_spotlighted,
                on_un_spotlighted,
                camera_follow,
                check_torch,
                on_torchlit,
                on_un_torchlit,
                enemy_size,
                enemy_health,
                player_health,
                update_vignette,
                enemy_spawner,
            ),
        )
            .chain()
            .run_if(resource_exists::<GameAssets>)
            .run_if(in_state(GameStateMachine::Level))
            .in_set(PausableSystems),
    );
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
struct Spotlighted;

#[derive(Component, Reflect)]
struct SpeedFactor(f32);

#[derive(Component, Reflect)]
struct Torch(f32);

#[derive(Component)]
struct Torchlit;

#[derive(Component, Reflect)]
struct Health(f32);

#[derive(Component)]
struct Vox;

#[derive(Component)]
struct EnemySpotlight;

#[derive(Component)]
struct EnemyTorchSpotlight;

// ==============================
// Mirror bounce (A + C)
// ==============================

#[derive(Component)]
struct Mirror {
    /// Mirror normal in local space (rotate by the entity's rotation to get world normal).
    local_normal: Vec3,
}

#[derive(Component)]
struct ReflectedSpotlight;

/// Mirror collision group for raycasts (so we only hit mirrors)
const MIRROR_GROUP: Group = Group::GROUP_2;

// ==============================
// Cursed controls
// ==============================

#[derive(Resource, Debug)]
pub struct CursedControls {
    pub enabled: bool,

    // Movement corruption
    pub speed_mul: f32,
    pub invert: Vec2,
    pub skew: Vec2,
    pub swirl_strength: f32,

    // Aim corruption
    pub aim_rotate_rad: f32,
    pub aim_wobble_rad: f32,
    pub aim_wobble_hz: f32,
    pub aim_lag: f32,    // lower = more lag
    pub aim_jitter: f32, // world-units jitter
}

impl Default for CursedControls {
    fn default() -> Self {
        Self {
            enabled: false,
            speed_mul: 1.8,
            invert: Vec2::new(-1.0, 1.0),
            skew: Vec2::new(0.65, -0.35),
            swirl_strength: 0.35,
            aim_rotate_rad: 0.9,
            aim_wobble_rad: 0.35,
            aim_wobble_hz: 1.7,
            aim_lag: 0.12,
            aim_jitter: 1.0,
        }
    }
}

#[derive(Resource, Default)]
pub struct CursedAimState {
    /// Smoothed/lagged aim direction (horizontal).
    pub current_dir: Vec3,
}

fn toggle_cursed_controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut cursed: ResMut<CursedControls>,
    mut aim_state: ResMut<CursedAimState>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
) {
    if !keys.just_pressed(KeyCode::KeyQ) {
        return;
    }

    cursed.enabled = !cursed.enabled;

    if cursed.enabled {
        cursed.speed_mul = rng.random_range(1.4..3.0);
        cursed.invert = Vec2::new(
            if rng.random_bool(0.5) { -1.0 } else { 1.0 },
            if rng.random_bool(0.5) { -1.0 } else { 1.0 },
        );
        cursed.skew = Vec2::new(rng.random_range(-1.0..1.0), rng.random_range(-1.0..1.0));
        cursed.swirl_strength = rng.random_range(0.15..0.75);

        cursed.aim_rotate_rad = rng.random_range(-std::f32::consts::PI..std::f32::consts::PI);
        cursed.aim_wobble_rad = rng.random_range(0.15..0.9);
        cursed.aim_wobble_hz = rng.random_range(0.6..3.5);
        cursed.aim_lag = rng.random_range(0.04..0.22);
        cursed.aim_jitter = rng.random_range(0.3..2.5);

        aim_state.current_dir = Vec3::ZERO;
        info!("Cursed controls ENABLED: {:?}", *cursed);
    } else {
        info!("Cursed controls disabled.");
    }
}

// ==============================
// Spawning
// ==============================

pub fn spawn_level(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    assets: Res<GameAssets>,
) {
    // Ground
    commands.spawn((
        DespawnOnExit(GameStateMachine::Level),
        DespawnOnExit(Screen::Gameplay),
        Visibility::default(),
        Mesh3d(meshes.add({
            let mut mesh = Plane3d::default().mesh().size(1000.0, 1000.0).build();
            if let Some(VertexAttributeValues::Float32x2(uvs)) =
                mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0)
            {
                for uv in uvs {
                    uv[0] *= 150.0;
                    uv[1] *= 150.0;
                }
            }
            mesh
        })),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(assets.grass_texture.clone()),
            reflectance: 0.0,
            ..default()
        })),
        Collider::cuboid(1000.0, 0.1, 1000.0),
        RigidBody::Fixed,
    ));

    // MIRROR (VERY visible)
    // Put it close so you cannot miss it.
    // let mirror_half_extents = Vec3::new(1.5, 2.0, 0.06);
    // let mirror_size = mirror_half_extents * 2.0;

    // let mirror_mesh = meshes.add(Cuboid::new(mirror_size.x, mirror_size.y, mirror_size.z));
    // let frame_mesh = meshes.add(Cuboid::new(
    //     mirror_size.x * 1.06,
    //     mirror_size.y * 1.06,
    //     mirror_size.z * 2.0,
    // ));

    // let mirror_mat = materials.add(StandardMaterial {
    //     base_color: Color::srgb(0.25, 0.28, 0.35),
    //     metallic: 1.0,
    //     perceptual_roughness: 0.12,
    //     reflectance: 1.0,
    //     emissive: Color::srgb(0.12, 0.22, 0.55).into(),
    //     ..default()
    // });

    // let frame_mat = materials.add(StandardMaterial {
    //     base_color: Color::srgb(0.02, 0.02, 0.03),
    //     emissive: Color::srgb(0.25, 0.55, 1.0).into(),
    //     metallic: 0.0,
    //     perceptual_roughness: 1.0,
    //     ..default()
    // });

    // let mirror_pos = Vec3::new(15., mirror_half_extents.y, -5.0);
    // let mirror_rot = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);

    // commands.spawn((
    //     Name::new("Mirror"),
    //     DespawnOnExit(GameStateMachine::Level),DespawnOnExit(Screen::Gameplay),
    //     Mirror {
    //         local_normal: Vec3::Z,
    //     },
    //     Transform::from_translation(mirror_pos).with_rotation(mirror_rot),
    //     GlobalTransform::default(),
    //     Visibility::default(),
    //     RigidBody::Fixed,
    //     Collider::cuboid(
    //         mirror_half_extents.x,
    //         mirror_half_extents.y,
    //         mirror_half_extents.z,
    //     ),
    //     CollisionGroups::new(MIRROR_GROUP, Group::ALL),
    //     // visuals + helper light
    //     children![
    //         (
    //             Mesh3d(frame_mesh),
    //             MeshMaterial3d(frame_mat),
    //             Transform::default()
    //         ),
    //         (
    //             Mesh3d(mirror_mesh),
    //             MeshMaterial3d(mirror_mat),
    //             Transform::default()
    //         ),
    //         (
    //             PointLight {
    //                 intensity: 2500.0,
    //                 range: 12.0,
    //                 color: LIGHT_COLOR,
    //                 ..default()
    //             },
    //             Transform::from_xyz(0.0, 0.0, 0.8)
    //         )
    //     ],
    // ));

    // Reflected spotlight (single entity, toggled visible when player light hits mirror)
    commands.spawn((
        Name::new("Reflected Spotlight"),
        DespawnOnExit(GameStateMachine::Level),
        DespawnOnExit(Screen::Gameplay),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Visibility::default(),
        children![
            (
                ReflectedSpotlight,
                SpotLight {
                    color: MIRROR_COLOR,
                    ..default()
                },
                Transform::from_xyz(0.0, 0.2, 0.0),
                Visibility::Hidden,
            ),
            (
                ReflectedSpotlight,
                SpotLight {
                    color: MIRROR_COLOR,
                    ..default()
                },
                Transform::from_xyz(0.0, 2.0, 0.0),
                Visibility::Hidden,
            )
        ],
    ));

    // Torch
    // let torch_range = 4.;
    // commands.spawn((
    //     Name::new("Torch"),
    //     DespawnOnExit(GameStateMachine::Level),DespawnOnExit(Screen::Gameplay),
    //     Visibility::default(),
    //     Transform::from_xyz(3.0, 0., 3.0),
    //     Torch(torch_range),
    //     RigidBody::Fixed,
    //     Collider::cuboid(0.5, 0.5, 0.5),
    //     children![
    //         (
    //             Visibility::default(),
    //             SceneRoot(assets.lamp.clone()),
    //             Transform::from_scale(vec3(0.2, 0.2, 0.2)),
    //         ),
    //         (
    //             Transform::from_xyz(0.0, 2.5, 0.0),
    //             PointLight {
    //                 color: TORCH_COLOR,
    //                 intensity: 100000.0,
    //                 range: torch_range + 3.,
    //                 radius: std::f32::consts::PI,
    //                 ..default()
    //             },
    //         )
    //     ],
    // ));

    // Player
    let flashlight_range = 6.0;
    let flashlight_angle = 0.2;
    commands.spawn((
        Name::new("Player"),
        DespawnOnExit(GameStateMachine::Level),
        DespawnOnExit(Screen::Gameplay),
        SpeedFactor(3.),
        Health(100.),
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
        Visibility::default(),
        RigidBody::KinematicPositionBased,
        Collider::cuboid(0.5, 0.5, 0.5),
        Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)),
        KinematicCharacterController::default(),
        children![
            (
                Name::new("Player Spotlight"),
                DespawnOnExit(GameStateMachine::Level),
                DespawnOnExit(Screen::Gameplay),
                PlayerSpotlight,
                Transform::from_xyz(0.0, -0.8, 0.0),
                SpotLight {
                    color: LIGHT_COLOR,
                    outer_angle: flashlight_angle,
                    inner_angle: flashlight_angle - 0.1,
                    range: flashlight_range,
                    intensity: 500000.0,
                    ..default()
                },
            ),
            (
                Name::new("Player Spotlight2"),
                DespawnOnExit(GameStateMachine::Level),
                DespawnOnExit(Screen::Gameplay),
                Transform::from_xyz(0.0, 2., 0.0),
                SpotLight {
                    color: LIGHT_COLOR,
                    outer_angle: flashlight_angle,
                    inner_angle: flashlight_angle - 0.1,
                    range: flashlight_range,
                    intensity: 500000.0,
                    ..default()
                },
            ),
            (
                Name::new("Player Down Spotlight"),
                DespawnOnExit(GameStateMachine::Level),
                DespawnOnExit(Screen::Gameplay),
                Transform::from_xyz(0.0, 5.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
                SpotLight {
                    color: LIGHT_COLOR,
                    outer_angle: 1.,
                    range: 8.,
                    intensity: 100000.0,
                    ..default()
                },
            ),
            (
                Name::new("Player Vox"),
                DespawnOnExit(GameStateMachine::Level),
                DespawnOnExit(Screen::Gameplay),
                Visibility::default(),
                SceneRoot(assets.vox0.clone()),
                Vox,
                Transform::from_scale(vec3(0.125, 0.06, 0.125))
                    .with_translation(vec3(-0.9, -1., -0.5)),
            ),
        ],
    ));
}

// ==============================
// Spotlight/torch visual toggles
// ==============================

fn on_un_spotlighted(
    mut removed: RemovedComponents<Spotlighted>,
    enemies: Query<&Children>,
    mut enemy_spotlights: Query<&mut Visibility, With<EnemySpotlight>>,
) {
    for e in removed.read() {
        if let Ok(children) = enemies.get(e) {
            for &child in children {
                if let Ok(mut light) = enemy_spotlights.get_mut(child) {
                    *light = Visibility::Hidden;
                }
            }
        }
    }
}

fn on_spotlighted(
    enemies: Query<&Children, (With<Enemy>, Added<Spotlighted>)>,
    mut enemy_spotlights: Query<&mut Visibility, With<EnemySpotlight>>,
) {
    for children in &enemies {
        for &child in children {
            if let Ok(mut light) = enemy_spotlights.get_mut(child) {
                *light = Visibility::Visible;
            }
        }
    }
}

fn on_un_torchlit(
    mut removed: RemovedComponents<Torchlit>,
    enemies: Query<&Children>,
    mut enemy_spotlights: Query<&mut Visibility, With<EnemyTorchSpotlight>>,
) {
    for e in removed.read() {
        if let Ok(children) = enemies.get(e) {
            for &child in children {
                if let Ok(mut light) = enemy_spotlights.get_mut(child) {
                    *light = Visibility::Hidden;
                }
            }
        }
    }
}

fn on_torchlit(
    enemies: Query<&Children, (With<Enemy>, Added<Torchlit>)>,
    mut enemy_spotlights: Query<&mut Visibility, With<EnemyTorchSpotlight>>,
) {
    for children in &enemies {
        for &child in children {
            if let Ok(mut light) = enemy_spotlights.get_mut(child) {
                *light = Visibility::Visible;
            }
        }
    }
}

// ==============================
// Player movement + cursed movement
// ==============================

fn apply_movement(
    movement: On<Fire<Movement>>,
    mut controller: Single<&mut KinematicCharacterController>,
    player_speed: Single<&SpeedFactor, With<Player>>,
    time: Res<Time>,
    cursed: Res<CursedControls>,
) {
    let mut input = movement.value;

    if cursed.enabled {
        let x = (input.x * cursed.invert.x) + (input.y * cursed.skew.x);
        let y = (input.y * cursed.invert.y) + (input.x * cursed.skew.y);

        let t = time.elapsed_secs();
        let swirl = Vec2::new((t * 2.3).sin(), (t * 1.9).cos()) * cursed.swirl_strength;

        input = (Vec2::new(x, y) + swirl) * cursed.speed_mul;
    }

    let forward = Vec3::new(-1.0, 0.0, -1.0).normalize();
    let right = Vec3::new(1.0, 0.0, -1.0).normalize();

    // Intentionally not normalized: diagonals & cursed feel “oddly faster”
    let direction = forward * input.y + right * input.x;

    controller.translation = Some(direction * player_speed.0 * time.delta_secs());
}

// ==============================
// Player aim + cursed aim
// ==============================

fn aim_spotlight(
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform)>,
    mut player: Single<&mut Transform, With<Player>>,
    time: Res<Time>,
    cursed: Res<CursedControls>,
    mut aim_state: ResMut<CursedAimState>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
) {
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let Ok(ray) = camera.0.viewport_to_world(camera.1, cursor_pos) else {
        return;
    };

    let denom = ray.direction.y;
    if denom.abs() <= 1e-6 {
        return;
    }
    let t = (0.0 - ray.origin.y) / denom;
    if t < 0.0 {
        return;
    }

    let mut target = ray.origin + *ray.direction * t;

    if cursed.enabled {
        let wobble = (time.elapsed_secs() * cursed.aim_wobble_hz * std::f32::consts::TAU).sin()
            * cursed.aim_wobble_rad;
        let angle = cursed.aim_rotate_rad + wobble;

        let p = player.translation;
        let v = target - p;
        target = p + Quat::from_rotation_y(angle) * v;

        if rng.random_bool(0.08) {
            let jx = rng.random_range(-cursed.aim_jitter..cursed.aim_jitter);
            let jz = rng.random_range(-cursed.aim_jitter..cursed.aim_jitter);
            target += Vec3::new(jx, 0.0, jz);
        }
    }

    let player_pos = player.translation;
    let mut dir = target - player_pos;
    dir.y = 0.0;

    let desired = dir.normalize_or_zero();
    if desired == Vec3::ZERO {
        return;
    }

    let final_dir = if cursed.enabled {
        if aim_state.current_dir == Vec3::ZERO {
            aim_state.current_dir = desired;
        } else {
            let alpha = (cursed.aim_lag * 60.0 * time.delta_secs()).clamp(0.01, 0.35);
            aim_state.current_dir = aim_state
                .current_dir
                .lerp(desired, alpha)
                .normalize_or_zero();
        }
        aim_state.current_dir
    } else {
        aim_state.current_dir = Vec3::ZERO;
        desired
    };

    if final_dir != Vec3::ZERO {
        player.look_to(final_dir, Vec3::Y);
    }
}

// ==============================
// Mirror reflection update (A + C)
// ==============================

fn update_reflected_spotlight(
    rapier_context: ReadRapierContext,

    // READ player spotlight; must be disjoint from the reflected spotlight
    player_light: Single<
        (&GlobalTransform, &SpotLight),
        (With<PlayerSpotlight>, Without<ReflectedSpotlight>),
    >,

    mirrors: Query<(&GlobalTransform, &Mirror)>,

    // WRITE reflected spotlight; must be disjoint from the player spotlight
    mut reflected_query: Query<
        (&mut Transform, &mut SpotLight, &mut Visibility),
        (With<ReflectedSpotlight>, Without<PlayerSpotlight>),
    >,
) {
    let rapier = rapier_context.single().unwrap();

    let (light_xform, light) = *player_light;
    let origin = light_xform.translation();
    let dir = light_xform.forward().normalize();

    // Raycast ONLY against mirrors
    let filter = QueryFilter::default().groups(CollisionGroups::new(Group::ALL, MIRROR_GROUP));
    for (mut t, mut refl_light, mut vis) in reflected_query.iter_mut() {
        let Some((hit_entity, toi)) = rapier.cast_ray(origin, dir, light.range, true, filter)
        else {
            *vis = Visibility::Hidden;
            continue;
        };

        let Ok((mirror_xform, mirror)) = mirrors.get(hit_entity) else {
            *vis = Visibility::Hidden;
            continue;
        };

        let hit_point = origin + dir * toi;

        let n = (mirror_xform.rotation() * mirror.local_normal).normalize();
        let r = (dir - 2.0 * dir.dot(n) * n).normalize_or_zero();
        if r == Vec3::ZERO {
            *vis = Visibility::Hidden;
            continue;
        }

        let spawn_pos = hit_point + r * 0.15;

        t.translation = spawn_pos;
        t.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, r);

        refl_light.inner_angle = light.inner_angle;
        refl_light.outer_angle = light.outer_angle;
        refl_light.range = light.range * 2.;
        refl_light.intensity = light.intensity * 1.5;

        *vis = Visibility::Visible;
    }
}

// ==============================
// Torch check (refactored: reuse allocations)
// ==============================

fn check_torch(
    mut commands: Commands,
    enemies: Query<(Entity, &GlobalTransform), With<Enemy>>,
    torches: Query<(&GlobalTransform, &Torch)>,
    mut hit_enemies: Local<HashSet<Entity>>,
) {
    hit_enemies.clear();

    for (torch_transform, torch) in &torches {
        let torch_pos = torch_transform.translation();
        let range = torch.0;

        for (entity, enemy_transform) in &enemies {
            let d = torch_pos.distance(enemy_transform.translation());
            if d <= range {
                hit_enemies.insert(entity);
            }
        }
    }

    for (entity, _) in &enemies {
        if hit_enemies.contains(&entity) {
            commands.entity(entity).try_insert(Torchlit);
        } else {
            commands.entity(entity).try_remove::<Torchlit>();
        }
    }
}

// ==============================
// Spotlight check (includes reflected spotlight for gameplay)
// ==============================

fn check_spotlight(
    mut commands: Commands,
    rapier_context: ReadRapierContext,
    enemies: Query<(Entity, &GlobalTransform), With<Enemy>>,
    spotlights: Query<
        (&GlobalTransform, &SpotLight, &Visibility),
        Or<(With<PlayerSpotlight>, With<ReflectedSpotlight>)>,
    >,
    mut hit_enemies: Local<HashSet<Entity>>,
    mut cached_cone: Local<Option<(f32, f32, Collider)>>, // (range, outer_angle, collider)
) {
    let rapier_context = rapier_context.single().unwrap();
    hit_enemies.clear();

    for (spotlight_transform, spotlight, vis) in &spotlights {
        if matches!(*vis, Visibility::Hidden) {
            continue;
        }

        let range = spotlight.range;
        let outer = spotlight.outer_angle;

        let shape = match cached_cone.as_ref() {
            Some((r, o, c)) if (*r - range).abs() < 1e-6 && (*o - outer).abs() < 1e-6 => c,
            _ => {
                let cone_half_height = range / 2.0;
                let cone_radius = range * outer.tan();
                let cone = Collider::cone(cone_half_height, cone_radius);
                *cached_cone = Some((range, outer, cone));
                &cached_cone.as_ref().unwrap().2
            }
        };

        let ray_dir = spotlight_transform.forward().normalize();
        let cone_half_height = range / 2.0;

        let shape_pos = spotlight_transform.translation() + ray_dir * cone_half_height;
        let shape_rot = Quat::from_rotation_arc(Vec3::Y, -ray_dir);

        let filter = QueryFilter::default().exclude_sensors();

        rapier_context.intersect_shape(
            shape_pos,
            shape_rot,
            shape.raw.as_ref(),
            filter,
            |entity| {
                if enemies.get(entity).is_ok() {
                    hit_enemies.insert(entity);
                }
                true
            },
        );
    }

    for (entity, _) in &enemies {
        if hit_enemies.contains(&entity) {
            commands.entity(entity).try_insert(Spotlighted);
        } else {
            commands.entity(entity).try_remove::<Spotlighted>();
        }
    }
}

// ==============================
// Enemy behavior
// ==============================

fn enemy_chase_player(
    player: Single<&Transform, (With<Player>, Without<Enemy>)>,
    mut enemies: Query<
        (
            &mut Transform,
            &mut ExternalForce,
            &Velocity,
            &SpeedFactor,
            Has<Spotlighted>,
            Has<Torchlit>,
        ),
        With<Enemy>,
    >,
) {
    let player_pos = player.translation;

    for (mut enemy_transform, mut ext_force, velocity, speed_factor, is_spotlighted, is_torchlit) in
        &mut enemies
    {
        enemy_transform.look_at(player_pos, Vec3::Y);

        if is_spotlighted || is_torchlit {
            ext_force.force = Vec3::ZERO;
            continue;
        }

        let direction = (player_pos - enemy_transform.translation) * Vec3::new(1.0, 0.0, 1.0);
        if direction.length_squared() > 0.01 {
            let desired_vel = direction.normalize() * speed_factor.0;
            let force_strength = 20.0;
            ext_force.force = (desired_vel - velocity.linvel) * force_strength;
            ext_force.force.y = 0.0;
        }
    }
}

fn calc_size(health: f32) -> f32 {
    let t = (health / 100.0).clamp(0.0, 1.0);
    let min_size: f32 = 0.3;
    let max_size: f32 = 1.;
    // Use a square root curve allowing enemies to be larger at low health
    min_size + (max_size - min_size) * t.sqrt()
}

fn enemy_size(
    enemies: Query<(&Health, &Children), (With<Enemy>, Changed<Health>)>,
    children2_query: Query<&Children>,
    mut vox: Query<&mut Transform, With<Vox>>,
) {
    for (health, children) in enemies.iter() {
        let scale = calc_size(health.0);
        for child in children {
            if let Ok(children2) = children2_query.get(*child) {
                for child2 in children2 {
                    if let Ok(mut transform) = vox.get_mut(*child2) {
                        transform.scale = Vec3::splat(scale);
                    }
                }
            }
        }
    }
}

fn enemy_health(
    mut commands: Commands,
    mut enemies: Query<
        (Entity, &mut Health),
        (With<Enemy>, Or<(With<Spotlighted>, With<Torchlit>)>),
    >,
    time: Res<Time>,
    mut game_state: ResMut<GameState>,
) {
    for (entity, mut health) in enemies.iter_mut() {
        health.0 -= time.delta_secs() * 25.0;
        if health.0 <= 0.0 {
            game_state.kills_this_night += 1;
            game_state.total_kills += 1;
            commands.entity(entity).despawn();
        }
    }
}

fn player_health(
    mut player: Single<(&Transform, &mut Health, &Player)>,
    enemies: Query<&Transform, (With<Enemy>, Without<Player>)>,
    time: Res<Time>,
    mut game_state: ResMut<NextState<GameStateMachine>>,
) {
    for enemy_transform in enemies {
        let distance = player.0.translation.distance(enemy_transform.translation);
        if distance < 6.0 {
            let t = 1.0 - (distance / 6.0);
            let damage_factor = 25.0 * t.powi(2);
            player.1.0 -= time.delta_secs() * damage_factor;
            player.1.0 = player.1.0.max(0.0);
        }
    }
    if player.1.0 <= 0.0 {
        game_state.set(GameStateMachine::Shop);
    }
}

fn update_vignette(player: Single<&Health, With<Player>>, mut camera: Single<&mut CrtSettings>) {
    let health = (player.0 / 100.0).clamp(0.0, 1.0);
    // Scale from 0.5 -> 10 as health goes from 100 -> 0 but ramp towards 10 as we get closer to 0 health
    camera.vignette_intensity = 0.5 + 10.0 * (1.0 - health).powi(2);
    if player.0 <= 0. {
        camera.brightness = 0.;
    } else {
        camera.brightness = 6.0 - 5.0 * (1.0 - health).powi(2);
    }
}

// ==============================
// Camera follow
// ==============================

fn camera_follow(
    player: Single<&Transform, (With<Player>, Changed<Transform>)>,
    mut camera: Single<
        (&mut Transform, &IsometricCamera),
        (Without<Player>, Without<PlayerSpotlight>),
    >,
    time: Res<Time>,
) {
    let (ref mut cam_transform, iso_cam) = *camera;
    let target_pos = player.translation + iso_cam.offset;

    let smoothness = 8.0;
    cam_transform.translation = cam_transform
        .translation
        .lerp(target_pos, smoothness * time.delta_secs());
}

fn enemy_spawner(
    mut commands: Commands,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
    assets: Res<GameAssets>,
    enemies: Query<Entity, With<Enemy>>,
    player_transform: Single<&Transform, With<Player>>,
    game_state: Res<GameState>,
) {
    if game_state.night_number == 1 && game_state.kills_this_night == 0 {
        if enemies.is_empty() {
            spawn_enemy(
                &mut commands,
                -10.0,
                -10.0,
                assets.vox5.clone(),
                3.0,
                60.0,
                0.6,
            );
        }
    } else {
        let total_enemies = 10 + (game_state.survived_seconds_this_night / 5.0).floor() as usize;
        let enemies_to_spawn = total_enemies - enemies.count();

        for _ in 0..enemies_to_spawn.max(0) {
            let max_health = 10. + (game_state.survived_seconds_this_night / 5.0);
            let health = rng.random_range(5.0..max_health);
            let scale = calc_size(health);
            // Spawn behind the player
            let back = player_transform.rotation * Vec3::Z;
            let base_angle = back.z.atan2(back.x);
            let spread = std::f32::consts::PI;
            let theta = base_angle + rng.random_range(-spread..spread);

            let radius = rng.random_range(12.0..20.0);
            let x = player_transform.translation.x + radius * theta.cos();
            let z = player_transform.translation.z + radius * theta.sin();
            let speed_factor = rng.random_range(1.0..4.0);

            let vox = match rng.random_range(1..6) {
                1 => assets.vox1.clone(),
                2 => assets.vox2.clone(),
                3 => assets.vox3.clone(),
                4 => assets.vox4.clone(),
                5 => assets.vox5.clone(),
                _ => unreachable!(),
            };
            spawn_enemy(&mut commands, x, z, vox, speed_factor, health, scale);
        }
    }
}

fn tick_player_time(mut game_state: ResMut<GameState>, time: Res<Time>) {
    game_state.survived_seconds_this_night += time.delta_secs();
}

fn spawn_enemy(
    commands: &mut Commands,
    x: f32,
    z: f32,
    vox: Handle<Scene>,
    speed_factor: f32,
    health: f32,
    scale: f32,
) {
    commands
        .spawn((
            Visibility::default(),
            Enemy,
            Name::new("Enemy"),
            RigidBody::Dynamic,
            Collider::cuboid(0.5, 0.5, 0.5),
            Transform::from_translation(vec3(x, 1., z)),
            Velocity::default(),
            ExternalForce::default(),
            Damping {
                linear_damping: 5.0,
                angular_damping: 1.0,
            },
            LockedAxes::TRANSLATION_LOCKED_Y | LockedAxes::ROTATION_LOCKED,
            Ccd::enabled(),
            SpeedFactor(speed_factor),
            Health(health),
            children![
                (
                    Name::new("Enemy Vox"),
                    DespawnOnExit(GameStateMachine::Level),
                    DespawnOnExit(Screen::Gameplay),
                    Visibility::default(),
                    Transform::from_scale(vec3(
                        0.125 * 2. * scale,
                        0.06 * 2. * scale,
                        0.125 * 2. * scale
                    ))
                    .with_translation(vec3(-1., -1., -0.5)),
                    children![(SceneRoot(vox), Vox, Transform::default())]
                ),
                (
                    Name::new("Enemy Down Spotlight"),
                    DespawnOnExit(GameStateMachine::Level),
                    DespawnOnExit(Screen::Gameplay),
                    EnemySpotlight,
                    Visibility::Hidden,
                    Transform::from_xyz(0.0, 5.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
                    SpotLight {
                        color: LIGHT_COLOR,
                        outer_angle: 1.,
                        range: 8.,
                        intensity: 100000.0,
                        ..default()
                    },
                ),
                (
                    Name::new("Enemy Torchlit Spotlight"),
                    DespawnOnExit(GameStateMachine::Level),
                    DespawnOnExit(Screen::Gameplay),
                    EnemyTorchSpotlight,
                    Visibility::Hidden,
                    Transform::from_xyz(0.0, 5.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
                    SpotLight {
                        color: TORCH_COLOR,
                        outer_angle: 1.,
                        range: 8.,
                        intensity: 100000.0,
                        ..default()
                    },
                )
            ],
        ))
        .insert((
            DespawnOnExit(Screen::Gameplay),
            DespawnOnExit(GameStateMachine::Level),
        ));
}
