mod hud;
mod intro;
mod level;
mod shop;

use bevy::{
    image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
};

use crate::asset_tracking::LoadResource;

pub const LIGHT_COLOR: Color = Color::srgb(1., 195. / 255., 0.0);

#[derive(States, Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub enum GameStateMachine {
    #[default]
    Initial,
    Intro,
    Level,
    Shop,
}

#[derive(Resource, Debug, Reflect)]
pub struct GameState {
    night_number: usize,
    kills_this_night: usize,
    survived_seconds_this_night: f32,
    total_kills: usize,
}

pub(super) fn plugin(app: &mut App) {
    app.init_state::<GameStateMachine>();
    app.insert_resource(GameState {
        night_number: 1,
        kills_this_night: 0,
        survived_seconds_this_night: 0.0,
        total_kills: 0,
    });
    app.load_resource::<GameAssets>();
    app.add_plugins(intro::plugin);
    app.add_plugins(shop::plugin);
    app.add_plugins(level::plugin);
    app.add_plugins(hud::plugin);
}

pub fn start(mut state: ResMut<NextState<GameStateMachine>>) {
    state.set(GameStateMachine::Intro);
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
    #[dependency]
    lamp: Handle<Scene>,
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
            lamp: assets.load("vox/Lamp.vox"),
        }
    }
}
