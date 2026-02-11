// Support configuring Bevy lints within code.
#![cfg_attr(bevy_lint, feature(register_tool), register_tool(bevy))]
// Disable console on Windows for non-dev builds.
#![cfg_attr(not(feature = "dev"), windows_subsystem = "windows")]

mod asset_tracking;
#[cfg(feature = "dev")]
mod dev_tools;
mod game;
mod menus;
mod screens;
mod theme;

use bevy::{asset::AssetMetaCheck, camera::ScalingMode, prelude::*};
use bevy_rand::{plugin::EntropyPlugin, prelude::WyRand};
use bevy_rapier3d::prelude::*;
use bevy_seedling::SeedlingPlugin;

fn main() -> AppExit {
    App::new().add_plugins(AppPlugin).run()
}

pub struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        // Add Bevy plugins.
        app.add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    // Wasm builds will check for meta files (that don't exist) if this isn't set.
                    // This causes errors and even panics on web build on itch.
                    // See https://github.com/bevyengine/bevy_github_ci_template/issues/48.
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Window {
                        title: "Bevy Jam 7".to_string(),
                        fit_canvas_to_parent: true,
                        ..default()
                    }
                    .into(),
                    ..default()
                }),
        );

        app.add_plugins(SeedlingPlugin::default());
        app.add_plugins(RapierPhysicsPlugin::<NoUserData>::default());
        app.add_plugins(EntropyPlugin::<WyRand>::default());
        // app.add_plugins(RapierDebugRenderPlugin::default());

        // Add other plugins.
        app.add_plugins((
            asset_tracking::plugin,
            #[cfg(feature = "dev")]
            dev_tools::plugin,
            menus::plugin,
            screens::plugin,
            theme::plugin,
            game::plugin,
        ));

        // Set up the `Pause` state.
        app.init_state::<Pause>();
        app.configure_sets(Update, PausableSystems.run_if(in_state(Pause(false))));

        // Spawn the main camera.
        app.add_systems(Startup, spawn_camera);
    }
}

/// Whether or not the game is paused.
#[derive(States, Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
struct Pause(pub bool);

/// A system set for systems that shouldn't run while the game is paused.
#[derive(SystemSet, Copy, Clone, Eq, PartialEq, Hash, Debug)]
struct PausableSystems;

/// Marker for the main isometric camera. Stores the offset from the follow target.
#[derive(Component)]
pub struct IsometricCamera {
    pub offset: Vec3,
}

fn spawn_camera(mut commands: Commands) {
    let offset = Vec3::new(20.0, 20.0, 20.0);
    commands.spawn((
        Camera3d::default(),
        IsometricCamera { offset },
        AmbientLight {
            brightness: 100.0,
            ..default()
        },
        Projection::from(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: 30.0,
            },
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_xyz(offset.x, offset.y, offset.z).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
