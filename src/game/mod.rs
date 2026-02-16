mod dead;
mod end;
mod hud;
mod intro;
mod level;
mod shop;

use bevy::{
    image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
};
use bevy_rand::prelude::*;
use bevy_seedling::sample::AudioSample;
use rand::seq::SliceRandom;

use crate::{asset_tracking::LoadResource, quotes::QUOTES};

pub const LIGHT_COLOR: Color = Color::srgb(1., 195. / 255., 0.0);

#[derive(States, Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub enum GameStateMachine {
    #[default]
    Initial,
    Intro,
    Level,
    Dead,
    Shop,
    End,
}

#[derive(Resource, Debug, Reflect)]
pub struct GameState {
    night_number: usize,
    kills_this_night: usize,
    survived_seconds_this_night: f32,
    total_kills: usize,
    spent: usize,
    flashlight: Flashlight,
    torch: Option<Torch>,
    quotes: Vec<(String, String)>,
    current_quote_index: usize,
}

#[derive(Resource, Debug, Reflect)]
pub struct Flashlight {
    angle: f32,
    range: f32,
    intensity: f32,
    color: Color,
}

#[derive(Resource, Debug, Reflect)]
pub struct Torch {
    range: f32,
    on_seconds: f32,
    off_seconds: f32,
}

pub(super) fn plugin(app: &mut App) {
    app.insert_state::<GameStateMachine>(GameStateMachine::Initial);
    let mut quotes: Vec<(String, String)> =
        QUOTES.map(|[a, b]| (a.to_string(), b.to_string())).into();
    let mut rng = app
        .world_mut()
        .query_filtered::<&mut WyRand, With<GlobalRng>>();
    if let Ok(mut rng) = rng.single_mut(app.world_mut()) {
        quotes.shuffle(&mut rng);
    }

    app.insert_resource(GameState {
        night_number: 1,
        kills_this_night: 0,
        survived_seconds_this_night: 0.0,
        total_kills: 0,
        spent: 0,
        flashlight: Flashlight {
            angle: 0.35,
            range: 6.0,
            intensity: 500000.0,
            color: LIGHT_COLOR,
        },
        torch: None,
        quotes,
        current_quote_index: 0,
    });
    app.load_resource::<GameAssets>();
    app.add_plugins(intro::plugin);
    app.add_plugins(shop::plugin);
    app.add_plugins(level::plugin);
    app.add_plugins(hud::plugin);
    app.add_plugins(dead::plugin);
    app.add_plugins(end::plugin);
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
    #[dependency]
    pop_sound: Handle<AudioSample>,
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
            pop_sound: assets.load("audio/sound_effects/pop.ogg"),
        }
    }
}
