use bevy::{
    image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    window::PrimaryWindow,
};
use bevy_enhanced_input::prelude::*;
use bevy_mesh::VertexAttributeValues;
use bevy_rand::{global::GlobalRng, prelude::WyRand};
use bevy_rapier3d::prelude::*;
use rand::Rng;

use crate::{IsometricCamera, PausableSystems, asset_tracking::LoadResource, screens::Screen};

pub const LIGHT_COLOR: Color = Color::srgb(1., 195. / 255., 0.0);

pub(super) fn plugin(app: &mut App) {
    app.load_resource::<GameAssets>();
    app.add_plugins(EnhancedInputPlugin);
    app.add_input_context::<Player>();
    app.add_observer(apply_movement);
    app.add_systems(FixedUpdate, enemy_chase_player.in_set(PausableSystems));
    app.add_systems(
        Update,
        (
            aim_spotlight,
            check_spotlight,
            on_spotlighted,
            on_un_spotlighted,
            camera_follow,
        )
            .in_set(PausableSystems),
    );
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct GameAssets {
    #[dependency]
    grass_texture: Handle<Image>,
    #[dependency]
    vox0: Handle<Scene>,
    #[dependency]
    vox1: Handle<Scene>,
    #[dependency]
    vox2: Handle<Scene>,
    #[dependency]
    vox3: Handle<Scene>,
    #[dependency]
    vox4: Handle<Scene>,
    #[dependency]
    vox5: Handle<Scene>,
}

impl FromWorld for GameAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            grass_texture: assets.load_with_settings(
                "images/textures/planks.png",
                |settings: &mut ImageLoaderSettings| {
                    settings.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                        address_mode_u: ImageAddressMode::Repeat,
                        address_mode_v: ImageAddressMode::Repeat,
                        ..default()
                    });
                },
            ),
            vox0: assets.load("vox/Zeds-0-Zed_1.vox"),
            vox1: assets.load("vox/Zeds-1-Zed_2.vox"),
            vox2: assets.load("vox/Zeds-2-Zed_3.vox"),
            vox3: assets.load("vox/Zeds-3-Zed_4.vox"),
            vox4: assets.load("vox/Zeds-4-Zed_5.vox"),
            vox5: assets.load("vox/Zeds-5-Zed_6.vox"),
        }
    }
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

#[derive(Component)]
struct SpeedFactor(f32);

pub fn spawn_game(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
    assets: Res<GameAssets>,
) {
    commands.spawn((
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
        Collider::cuboid(1000.0, 0., 1000.0),
    ));
    commands.spawn((
        Name::new("Player"),
        DespawnOnExit(Screen::Gameplay),
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
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        KinematicCharacterController {
            apply_impulse_to_dynamic_bodies: true,
            ..KinematicCharacterController::default()
        },
        children![
            (
                Name::new("Player Spotlight"),
                DespawnOnExit(Screen::Gameplay),
                PlayerSpotlight,
                Transform::from_xyz(0.0, 0.2, 0.0),
                SpotLight {
                    color: LIGHT_COLOR,
                    outer_angle: 0.4,
                    inner_angle: 0.3,
                    range: 8.,
                    intensity: 10000000.0,
                    ..default()
                },
            ),
            (
                DespawnOnExit(Screen::Gameplay),
                SceneRoot(assets.vox0.clone()),
                Transform::from_scale(vec3(0.125, 0.06, 0.125))
                    .with_translation(vec3(-1., 0., -0.5))
            ),
            (
                Name::new("Player Down Spotlight"),
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
        ],
    ));
    for i in 0..10 {
        let x = rng.random_range(-50.0..50.0);
        let z = rng.random_range(-50.0..50.0);
        let speed_factor = rng.random_range(2.0..4.0);
        let vox = match rng.random_range(1..6) {
            1 => assets.vox1.clone(),
            2 => assets.vox2.clone(),
            3 => assets.vox3.clone(),
            4 => assets.vox4.clone(),
            5 => assets.vox5.clone(),
            _ => unreachable!(),
        };
        commands.spawn((
            DespawnOnExit(Screen::Gameplay),
            Visibility::default(),
            Enemy,
            Name::new(format!("Enemy {}", i)),
            RigidBody::Dynamic,
            Collider::cuboid(0.5, 0.5, 0.5),
            Transform::from_translation(vec3(x, 0., z)),
            Velocity::default(),
            ExternalForce::default(),
            Damping {
                linear_damping: 5.0,
                angular_damping: 1.0,
            },
            LockedAxes::TRANSLATION_LOCKED_Y | LockedAxes::ROTATION_LOCKED,
            Ccd::enabled(),
            SpeedFactor(speed_factor),
            children![
                (
                    DespawnOnExit(Screen::Gameplay),
                    SceneRoot(vox),
                    Transform::from_scale(vec3(0.125, 0.06, 0.125))
                        .with_translation(vec3(-1., 0., -0.5))
                ),
                (
                    Name::new("Enemy Down Spotlight"),
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
                )
            ],
        ));
    }
}

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

#[derive(Component)]
struct EnemySpotlight;

fn on_spotlighted(
    enemies: Query<&Children, (With<Enemy>, Added<Spotlighted>)>,
    mut enemy_spotlights: Query<&mut Visibility, With<EnemySpotlight>>,
) {
    for children in enemies {
        for &child in children {
            if let Ok(mut light) = enemy_spotlights.get_mut(child) {
                *light = Visibility::Visible;
            }
        }
    }
}

fn apply_movement(
    movement: On<Fire<Movement>>,
    mut controller: Single<&mut KinematicCharacterController>,
    time: Res<Time>,
) {
    let speed = 10.0;
    let input = movement.value;

    let forward = Vec3::new(-1.0, 0.0, -1.0).normalize();
    let right = Vec3::new(1.0, 0.0, -1.0).normalize();

    let direction = forward * input.y + right * input.x;

    controller.translation = Some(direction * speed * time.delta_secs());
}

fn aim_spotlight(
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform)>,
    mut player: Single<&mut Transform, With<Player>>,
) {
    if let Some(cursor_pos) = window.cursor_position()
        && let Ok(ray) = camera.0.viewport_to_world(camera.1, cursor_pos)
    {
        let ground_y = 0.0;
        let denom = ray.direction.y;
        if denom.abs() > 1e-6 {
            let t = (ground_y - ray.origin.y) / denom;
            if t >= 0.0 {
                let mouse_ground = ray.origin + *ray.direction * t;
                let player_pos = player.translation;

                let direction = mouse_ground - player_pos;
                let horizontal_direction =
                    Vec3::new(direction.x, 0.0, direction.z).normalize_or_zero();
                if horizontal_direction != Vec3::ZERO {
                    player.look_to(horizontal_direction, Vec3::Y);
                }
            }
        }
    }
}

fn check_spotlight(
    mut commands: Commands,
    rapier_context: ReadRapierContext,
    enemies: Query<Entity, With<Enemy>>,
    spotlights: Query<(&GlobalTransform, &SpotLight), With<PlayerSpotlight>>,
) {
    let rapier_context = rapier_context.single().unwrap();
    // Collect all enemies overlapping with the cone.
    let mut hit_enemies = std::collections::HashSet::new();
    for (spotlight_transform, spotlight) in spotlights {
        let ray_dir = spotlight_transform.forward().normalize();
        // Create a cone collider for the spotlight area.
        // half_height = how far the cone extends, radius = spread at the far end.
        let cone_half_height = spotlight.range / 2.0;
        //Use outer_angle to determine the radius
        let cone_radius = spotlight.range * spotlight.outer_angle.tan();
        let shape = Collider::cone(cone_half_height, cone_radius);

        // Position the cone so its center is ahead of the player along the aim direction.
        let shape_pos = spotlight_transform.translation() + ray_dir * cone_half_height;

        // Rotate so the cone's apex (default +Y) points back toward the player (-ray_dir),
        // meaning the wide base fans out in the ray_dir direction.
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
                true // keep searching
            },
        );
    }
    for entity in &enemies {
        if hit_enemies.contains(&entity) {
            commands.entity(entity).insert(Spotlighted);
        } else {
            commands.entity(entity).remove::<Spotlighted>();
        }
    }
}

fn enemy_chase_player(
    player: Single<&Transform, (With<Player>, Without<Enemy>)>,
    mut enemies: Query<
        (
            &mut Transform,
            &mut ExternalForce,
            &Velocity,
            &SpeedFactor,
            Has<Spotlighted>,
        ),
        With<Enemy>,
    >,
) {
    let player_pos = player.translation;

    for (mut enemy_transform, mut ext_force, velocity, speed_factor, is_spotlighted) in &mut enemies
    {
        enemy_transform.look_at(player_pos, Vec3::Y);
        if is_spotlighted {
            ext_force.force = Vec3::ZERO;
            continue;
        }
        let direction = (player_pos - enemy_transform.translation) * Vec3::new(1.0, 0.0, 1.0);
        if direction.length_squared() > 0.01 {
            let desired_vel = direction.normalize() * speed_factor.0;
            // Apply a force that steers toward the desired velocity, allowing
            // Rapier's collision solver to still push enemies apart.
            let force_strength = 20.0;
            ext_force.force = (desired_vel - velocity.linvel) * force_strength;
            ext_force.force.y = 0.0;
        }
    }
}

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

    // Smooth follow — adjust the speed factor to taste (higher = snappier)
    let smoothness = 8.0;
    cam_transform.translation = cam_transform
        .translation
        .lerp(target_pos, smoothness * time.delta_secs());
}
