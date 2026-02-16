use bevy::prelude::*;

use crate::{
    game::{GameState, GameStateMachine, Torch},
    screens::Screen,
    theme::widget,
};

#[derive(Component)]
struct FlashlightAngleText;

#[derive(Component)]
struct FlashlightAngleButton;

#[derive(Component)]
struct FlashlightRangeText;

#[derive(Component)]
struct FlashlightRangeButton;

#[derive(Component)]
struct BuyTorchUI;

#[derive(Component)]
struct UpgradeTorchUI;

#[derive(Component)]
struct TorchRangeText;

#[derive(Component)]
struct TorchRangeButton;

#[derive(Component)]
struct TorchOnSecondsText;

#[derive(Component)]
struct TorchOnSecondsButton;

#[derive(Component)]
struct TorchOffSecondsText;

#[derive(Component)]
struct TorchOffSecondsButton;

#[derive(Component)]
struct CurrencyText;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameStateMachine::Shop), spawn_shop);
    app.add_systems(
        Update,
        (update_torch_ui, update_flashlight_ui, update_currency)
            .run_if(in_state(GameStateMachine::Shop)),
    );
}

fn spawn_shop(mut commands: Commands) {
    commands.spawn((
        GlobalZIndex(1),
        DespawnOnExit(GameStateMachine::Shop),
        DespawnOnExit(Screen::Gameplay),
        Name::new("Shop"),
        Node {
            width: Val::Percent(80.0),
            height: Val::Percent(80.0),
            position_type: PositionType::Absolute,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: Val::Px(30.0),
            ..default()
        },
        children![
            (
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(30.0),
                    ..default()
                },
                Pickable::IGNORE,
                children![stats(), upgrades(),]
            ),
            (
                Node { ..default() },
                Pickable::IGNORE,
                children![widget::button("Next", go_to_intro),]
            ),
        ],
    ));
}

fn stats() -> impl Bundle {
    (
        Name::new("Stats"),
        Node {
            flex_direction: FlexDirection::Column,
            ..default()
        },
        children![(widget::header(""), CurrencyText),],
    )
}

fn upgrades() -> impl Bundle {
    (
        Name::new("Upgrades"),
        Node {
            flex_direction: FlexDirection::Column,
            flex_grow: 10.0,
            ..default()
        },
        BackgroundColor(Color::srgb(0.23, 0.23, 0.23)),
        children![
            widget::header("Upgrades"),
            widget::header(""),
            widget::header("Flashlight"),
            (
                Name::new("Angle"),
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(10),
                    ..default()
                },
                Pickable::IGNORE,
                Visibility::default(),
                children![
                    (widget::label(""), FlashlightAngleText,),
                    (
                        widget::button_small("+", increase_fashlight_angle),
                        FlashlightAngleButton,
                    ),
                ]
            ),
            (
                Name::new("Range"),
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(10),
                    ..default()
                },
                Pickable::IGNORE,
                Visibility::default(),
                children![
                    (widget::label(""), FlashlightRangeText,),
                    (
                        widget::button_small("+", increase_fashlight_range),
                        FlashlightRangeButton,
                    ),
                ]
            ),
            widget::label(""),
            widget::header("Torch"),
            (
                BuyTorchUI,
                Visibility::Hidden,
                widget::button("Buy a Torch (100g)", buy_torch)
            ),
            (
                UpgradeTorchUI,
                Visibility::Hidden,
                Node {
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                children![
                    (
                        Name::new("Range"),
                        Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: px(10),
                            ..default()
                        },
                        Pickable::IGNORE,
                        Visibility::default(),
                        children![
                            (widget::label(""), TorchRangeText,),
                            (
                                widget::button_small("+", increase_torch_range),
                                TorchRangeButton,
                            ),
                        ]
                    ),
                    (
                        Name::new("TorchOnSeconds"),
                        Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: px(10),
                            ..default()
                        },
                        Pickable::IGNORE,
                        Visibility::default(),
                        children![
                            (widget::label(""), TorchOnSecondsText,),
                            (
                                widget::button_small("+", increase_torch_on_seconds),
                                TorchOnSecondsButton,
                            ),
                        ]
                    ),
                    (
                        Name::new("TorchOffSeconds"),
                        Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: px(10),
                            ..default()
                        },
                        Pickable::IGNORE,
                        Visibility::default(),
                        children![
                            (widget::label(""), TorchOffSecondsText,),
                            (
                                widget::button_small("+", decrease_torch_off_seconds),
                                TorchOffSecondsButton,
                            ),
                        ]
                    )
                ],
            )
        ],
    )
}

fn go_to_intro(
    _: On<Pointer<Click>>,
    mut state: ResMut<NextState<GameStateMachine>>,
    mut game_state: ResMut<GameState>,
) {
    game_state.night_number += 1;
    game_state.kills_this_night = 0;
    game_state.survived_seconds_this_night = 0.0;
    state.set(GameStateMachine::Intro);
}

fn update_currency(
    game_state: Res<GameState>,
    mut currency_text: Single<&mut Text, With<CurrencyText>>,
) {
    **currency_text = format!("Currency: {}g", game_state.total_kills - game_state.spent).into();
}

fn update_flashlight_ui(
    game_state: ResMut<GameState>,
    mut flashlight_angle: Single<&mut Text, With<FlashlightAngleText>>,
    mut flashlight_range: Single<
        &mut Text,
        (With<FlashlightRangeText>, Without<FlashlightAngleText>),
    >,
    mut increase_flashlight_angle_button: Single<&mut Visibility, With<FlashlightAngleButton>>,
    mut increase_flashlight_range_button: Single<
        &mut Visibility,
        (With<FlashlightRangeButton>, Without<FlashlightAngleButton>),
    >,
) {
    **flashlight_angle = format!(
        "Angle: {:.0} degrees - (45g)",
        (2. * game_state.flashlight.angle.to_degrees()).floor()
    )
    .into();
    **flashlight_range = format!("Range: {:.0} - (45g)", game_state.flashlight.range).into();

    if max_flashlight_angle(&game_state) {
        **increase_flashlight_angle_button = Visibility::Hidden;
    }
    if max_flashlight_range(&game_state) {
        **increase_flashlight_range_button = Visibility::Hidden;
    }
}

fn update_torch_ui(
    game_state: Res<GameState>,
    mut torch_range: Single<&mut Text, With<TorchRangeText>>,
    mut torch_on_seconds: Single<&mut Text, (With<TorchOnSecondsText>, Without<TorchRangeText>)>,
    mut torch_off_seconds: Single<
        &mut Text,
        (
            With<TorchOffSecondsText>,
            Without<TorchRangeText>,
            Without<TorchOnSecondsText>,
        ),
    >,
    mut buy_torch_ui: Single<&mut Visibility, With<BuyTorchUI>>,
    mut upgrade_torch_ui: Single<&mut Visibility, (With<UpgradeTorchUI>, Without<BuyTorchUI>)>,
    mut increase_torch_range_button: Single<
        &mut Visibility,
        (
            With<TorchRangeButton>,
            Without<BuyTorchUI>,
            Without<UpgradeTorchUI>,
        ),
    >,
    mut decrease_torch_off_seconds_button: Single<
        &mut Visibility,
        (
            With<TorchOffSecondsButton>,
            Without<TorchRangeButton>,
            Without<BuyTorchUI>,
            Without<UpgradeTorchUI>,
        ),
    >,
) {
    match &game_state.torch {
        None => {
            **buy_torch_ui = Visibility::Visible;
            **upgrade_torch_ui = Visibility::Hidden;
        }
        Some(torch) => {
            **buy_torch_ui = Visibility::Hidden;
            **upgrade_torch_ui = Visibility::Visible;
            **torch_range = format!("Range: {:.0} - (45g)", torch.range).into();
            **torch_on_seconds = format!("Duration: {:.1} - (45g)", torch.on_seconds).into();
            **torch_off_seconds =
                format!("Cooldown Reduction: {:.1} - (45g)", torch.off_seconds).into();

            if max_torch_range(&game_state) {
                **increase_torch_range_button = Visibility::Hidden;
            }
            if max_torch_off_seconds(&game_state) {
                **decrease_torch_off_seconds_button = Visibility::Hidden;
            }
        }
    }
}

fn can_buy_falshlight_angle(game_state: &GameState) -> bool {
    !max_flashlight_angle(game_state) && game_state.total_kills - game_state.spent >= 45
}

fn max_flashlight_angle(game_state: &GameState) -> bool {
    2. * game_state.flashlight.angle.to_degrees() >= 97.0
}

fn increase_fashlight_angle(_: On<Pointer<Click>>, mut game_state: ResMut<GameState>) {
    if !can_buy_falshlight_angle(&game_state) {
        return;
    }
    game_state.flashlight.angle += 0.1;
    game_state.spent += 45;
}

fn can_buy_flashlight_range(game_state: &GameState) -> bool {
    !max_flashlight_range(game_state) && game_state.total_kills - game_state.spent >= 45
}

fn max_flashlight_range(game_state: &GameState) -> bool {
    game_state.flashlight.range >= 10.
}

fn increase_fashlight_range(_: On<Pointer<Click>>, mut game_state: ResMut<GameState>) {
    if !can_buy_flashlight_range(&game_state) {
        return;
    }
    game_state.flashlight.range += 1.0;
    game_state.spent += 45;
}

fn can_buy_torch(game_state: &GameState) -> bool {
    game_state.torch.is_none() && game_state.total_kills - game_state.spent >= 100
}

fn buy_torch(_: On<Pointer<Click>>, mut game_state: ResMut<GameState>) {
    if !can_buy_torch(&game_state) {
        return;
    }
    game_state.torch = Some(Torch {
        range: 5.0,
        on_seconds: 2.,
        off_seconds: 2.,
    });
    game_state.spent += 100;
}

fn max_torch_range(game_state: &GameState) -> bool {
    game_state.torch.as_ref().is_some_and(|t| t.range >= 10.0)
}

fn can_buy_torch_range(game_state: &GameState) -> bool {
    game_state.torch.is_some()
        && game_state.total_kills - game_state.spent >= 45
        && !max_torch_range(game_state)
}

fn increase_torch_range(_: On<Pointer<Click>>, mut game_state: ResMut<GameState>) {
    if !can_buy_torch_range(&game_state) {
        return;
    }
    game_state.torch.as_mut().unwrap().range += 1.0;
    game_state.spent += 45;
}

fn can_buy_torch_on_seconds(game_state: &GameState) -> bool {
    game_state.torch.is_some() && game_state.total_kills - game_state.spent >= 45
}

fn increase_torch_on_seconds(_: On<Pointer<Click>>, mut game_state: ResMut<GameState>) {
    if !can_buy_torch_on_seconds(&game_state) {
        return;
    }
    game_state.torch.as_mut().unwrap().on_seconds += 1.;
    game_state.spent += 45;
}

fn can_buy_torch_off_seconds(game_state: &GameState) -> bool {
    game_state.torch.is_some()
        && game_state.total_kills - game_state.spent >= 45
        && !max_torch_off_seconds(game_state)
}

fn max_torch_off_seconds(game_state: &GameState) -> bool {
    game_state
        .torch
        .as_ref()
        .is_some_and(|t| t.off_seconds <= 0.3)
}

fn decrease_torch_off_seconds(_: On<Pointer<Click>>, mut game_state: ResMut<GameState>) {
    if !can_buy_torch_off_seconds(&game_state) {
        return;
    }
    game_state.torch.as_mut().unwrap().off_seconds -= 0.1;
    game_state.spent += 45;
}
