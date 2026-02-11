use bevy::{
    ecs::system::IntoObserverSystem, input::common_conditions::input_just_pressed, prelude::*,
};
use bevy_seedling::{
    pool::SamplerPool,
    prelude::{MainBus, MusicPool, PerceptualVolume, SoundEffectsBus, Volume, VolumeNode},
    sample::{AudioSample, SamplePlayer},
};

use crate::{asset_tracking::LoadResource, menus::Menu, screens::Screen};

pub(super) fn plugin(app: &mut App) {
    app.load_resource::<SettingsAssets>();
    app.add_systems(OnEnter(Menu::Settings), spawn_settings_menu)
        .add_systems(
            Update,
            go_back.run_if(in_state(Menu::Settings).and(input_just_pressed(KeyCode::Escape))),
        )
        .add_systems(
            Update,
            (
                update_music_volume_label,
                update_master_volume_label,
                update_sfx_volume_label,
                button_hover,
            )
                .run_if(in_state(Menu::Settings)),
        );
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct SettingsAssets {
    #[dependency]
    music: Handle<AudioSample>,
    #[dependency]
    sfx: Handle<AudioSample>,
}

impl FromWorld for SettingsAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            music: assets.load("audio/music/selfless_courage.ogg"),
            sfx: assets.load("audio/sound_effects/crowd.ogg"),
        }
    }
}

fn spawn_settings_menu(mut commands: Commands) {
    commands.spawn((
        DespawnOnExit(Menu::Settings),
        GlobalZIndex(2),
        BackgroundColor(Color::srgb(0.23, 0.23, 0.23)),
        Node {
            width: Val::Percent(80.0),
            height: Val::Percent(80.0),
            position_type: PositionType::Absolute,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: Val::Vh(8.0),
            margin: UiRect::AUTO,
            padding: UiRect::axes(Val::Px(50.0), Val::Px(50.0)),
            border: UiRect::axes(Val::Px(2.0), Val::Px(2.0)),
            border_radius: BorderRadius::all(Val::Px(25.0)),
            ..default()
        },
        BorderColor::all(Color::srgb(0.9, 0.9, 0.9)),
        children![
            text((
                Text::new("Sound Settings"),
                TextFont {
                    font_size: 32.0,
                    ..Default::default()
                },
            )),
            core_grid(),
            play_buttons(),
            back_button(),
        ],
    ));
}

fn play_music(
    _: On<Pointer<Click>>,
    playing: Query<(), (With<MusicPool>, With<SamplePlayer>)>,
    mut commands: Commands,
    settings_assets: Res<SettingsAssets>,
) {
    // We'll only play music if it's not already playing.
    if playing.iter().len() > 0 {
        return;
    }

    commands.spawn((
        // Including the `MusicPool` marker queues this sample in the `MusicPool`.
        MusicPool,
        SamplePlayer::new(settings_assets.music.clone()).with_volume(Volume::Decibels(-6.0)),
    ));
}

pub fn play_sfx(
    _: On<Pointer<Click>>,
    mut commands: Commands,
    settings_assets: Res<SettingsAssets>,
) {
    // The default pool is routed to the `SoundEffectsBus`, so we don't
    // need to include any special markers for sound effects.
    commands.spawn(SamplePlayer::new(settings_assets.sfx.clone()));
}

//  ============================ Control Knob Observers ============================ //

const CONVERTER: PerceptualVolume = PerceptualVolume::new();

const MIN_VOLUME: f32 = 0.0;
const MAX_VOLUME: f32 = 2.0;
const STEP: f32 = 0.1;

fn increment_volume(volume: Volume) -> Volume {
    let perceptual = CONVERTER.volume_to_perceptual(volume);
    let new_perceptual = (perceptual + STEP).min(MAX_VOLUME);
    CONVERTER.perceptual_to_volume(new_perceptual)
}

fn decrement_volume(volume: Volume) -> Volume {
    let perceptual = CONVERTER.volume_to_perceptual(volume);
    let new_perceptual = (perceptual - STEP).max(MIN_VOLUME);
    CONVERTER.perceptual_to_volume(new_perceptual)
}

// Master
fn lower_master(_: On<Pointer<Click>>, mut master: Single<&mut VolumeNode, With<MainBus>>) {
    master.volume = decrement_volume(master.volume);
}

fn raise_master(_: On<Pointer<Click>>, mut master: Single<&mut VolumeNode, With<MainBus>>) {
    master.volume = increment_volume(master.volume);
}

fn update_master_volume_label(
    mut label: Single<&mut Text, With<MasterVolumeLabel>>,
    master: Single<&VolumeNode, (With<MainBus>, Changed<VolumeNode>)>,
) {
    let percent = CONVERTER.volume_to_perceptual(master.volume) * 100.0;
    let text = format!("{}%", percent.round());
    label.0 = text;
}

// Music
fn lower_music(
    _: On<Pointer<Click>>,
    mut music: Single<&mut VolumeNode, With<SamplerPool<MusicPool>>>,
) {
    music.volume = decrement_volume(music.volume);
}

fn raise_music(
    _: On<Pointer<Click>>,
    mut music: Single<&mut VolumeNode, With<SamplerPool<MusicPool>>>,
) {
    music.volume = increment_volume(music.volume);
}

fn update_music_volume_label(
    mut label: Single<&mut Text, With<MusicVolumeLabel>>,
    music: Single<&VolumeNode, With<SamplerPool<MusicPool>>>,
) {
    let percent = CONVERTER.volume_to_perceptual(music.volume) * 100.0;
    let text = format!("{}%", percent.round());
    label.0 = text;
}

// SFX
fn lower_sfx(_: On<Pointer<Click>>, mut sfx: Single<&mut VolumeNode, With<SoundEffectsBus>>) {
    sfx.volume = decrement_volume(sfx.volume);
}

fn raise_sfx(_: On<Pointer<Click>>, mut sfx: Single<&mut VolumeNode, With<SoundEffectsBus>>) {
    sfx.volume = increment_volume(sfx.volume);
}

fn update_sfx_volume_label(
    mut label: Single<&mut Text, With<SfxVolumeLabel>>,
    sfx: Single<&VolumeNode, With<SoundEffectsBus>>,
) {
    let percent = CONVERTER.volume_to_perceptual(sfx.volume) * 100.0;
    let text = format!("{}%", percent.round());
    label.0 = text;
}

//  ============================ UI Code ============================ //

fn core_grid() -> impl Bundle {
    (
        Name::new("Sound Grid"),
        Node {
            row_gap: Val::Px(10.0),
            column_gap: Val::Px(30.0),
            display: Display::Grid,
            width: Val::Percent(100.0),
            grid_template_columns: RepeatedGridTrack::percent(2, 50.0),
            ..default()
        },
        children![
            text(Text::new("Master")),
            master_volume(),
            text(Text::new("Music")),
            music_volume(),
            text(Text::new("Sfx")),
            sfx_volume(),
        ],
    )
}

fn play_buttons() -> impl Bundle {
    (
        Node {
            justify_content: JustifyContent::SpaceAround,
            width: Val::Percent(100.0),
            ..Default::default()
        },
        children![btn("Play Music", play_music), btn("Play Sfx", play_sfx),],
    )
}

fn master_volume() -> impl Bundle {
    (
        knobs_container(),
        children![
            btn("-", lower_master),
            knob_label(MasterVolumeLabel),
            btn("+", raise_master),
        ],
    )
}

fn back_button() -> impl Bundle {
    (
        Node {
            justify_content: JustifyContent::SpaceAround,
            width: Val::Percent(100.0),
            ..Default::default()
        },
        children![btn("Back", go_back_on_click),],
    )
}

#[derive(Component, Reflect)]
#[reflect(Component)]
struct MasterVolumeLabel;

fn music_volume() -> impl Bundle {
    (
        knobs_container(),
        children![
            btn("-", lower_music),
            knob_label(MusicVolumeLabel),
            btn("+", raise_music),
        ],
    )
}

#[derive(Component, Reflect)]
#[reflect(Component)]
struct MusicVolumeLabel;

fn sfx_volume() -> impl Bundle {
    (
        knobs_container(),
        children![
            btn("-", lower_sfx),
            knob_label(SfxVolumeLabel),
            btn("+", raise_sfx),
        ],
    )
}

#[derive(Component, Reflect)]
#[reflect(Component)]
struct SfxVolumeLabel;

pub fn btn<E, B, M, I>(t: impl Into<String>, action: I) -> impl Bundle
where
    E: EntityEvent,
    B: Bundle,
    I: IntoObserverSystem<E, B, M>,
{
    let action = IntoObserverSystem::into_system(action);
    let t: String = t.into();

    (
        Name::new("Button"),
        Node::default(),
        Children::spawn(SpawnWith(move |parent: &mut ChildSpawner| {
            parent
                .spawn((
                    Button,
                    BorderColor::all(Color::WHITE),
                    children![Name::new("Button text"), text(Text(t))],
                ))
                .observe(action);
        })),
    )
}

pub fn text(text: impl Bundle) -> impl Bundle {
    (
        Node {
            padding: UiRect::axes(Val::Px(10.0), Val::Px(10.0)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::all(Val::Percent(10.0)),
            ..Default::default()
        },
        BackgroundColor(Color::srgb(0.9, 0.9, 0.9)),
        children![(text, TextColor(Color::srgb(0.1, 0.1, 0.1)))],
    )
}

fn knobs_container() -> impl Bundle {
    Node {
        justify_self: JustifySelf::Center,
        align_content: AlignContent::SpaceEvenly,
        min_width: Val::Px(100.0),
        ..Default::default()
    }
}

fn knob_label(label: impl Component) -> impl Bundle {
    (
        Node {
            padding: UiRect::horizontal(Val::Px(10.0)),
            justify_content: JustifyContent::Center,
            ..Default::default()
        },
        children![text((
            Text::new(""),
            Node {
                min_width: Val::Px(75.0),
                ..Default::default()
            },
            TextLayout {
                justify: Justify::Center,
                ..Default::default()
            },
            label
        ))],
    )
}

const NORMAL_BUTTON: Color = Color::srgb(0.9, 0.9, 0.9);
const HOVERED_BUTTON: Color = Color::srgb(0.7, 0.7, 0.7);

fn button_hover(
    interaction_query: Query<(&Interaction, &Children), (Changed<Interaction>, With<Button>)>,
    mut text: Query<&mut BackgroundColor>,
) {
    for (interaction, children) in &interaction_query {
        let Some(mut color) = children.get(1).and_then(|c| text.get_mut(*c).ok()) else {
            continue;
        };

        match *interaction {
            Interaction::Pressed => {
                *color = NORMAL_BUTTON.into();
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
            }
        }
    }
}

fn go_back_on_click(
    _: On<Pointer<Click>>,
    screen: Res<State<Screen>>,
    next_menu: ResMut<NextState<Menu>>,
) {
    go_back(screen, next_menu);
}

fn go_back(screen: Res<State<Screen>>, mut next_menu: ResMut<NextState<Menu>>) {
    next_menu.set(if screen.get() == &Screen::Title {
        Menu::Main
    } else {
        Menu::Pause
    });
}
