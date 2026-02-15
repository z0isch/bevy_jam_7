use bevy::prelude::*;

use crate::{
    game::{GameState, GameStateMachine},
    screens::Screen,
    theme::widget,
};

#[derive(Component)]
struct TimeUI;

#[derive(Component)]
struct KillsUI;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameStateMachine::Level), spawn_hud);
    app.add_systems(Update, update_hud);
}

fn spawn_hud(mut commands: Commands) {
    commands.spawn((
        GlobalZIndex(1),
        DespawnOnExit(GameStateMachine::Level),
        DespawnOnExit(Screen::Gameplay),
        Name::new("HUD"),
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::ColumnReverse,
            ..default()
        },
        Pickable::IGNORE,
        children![(TimeUI, widget::header("")), (KillsUI, widget::header(""))],
    ));
}

fn update_hud(
    mut time: Single<&mut Text, With<TimeUI>>,
    mut kills: Single<&mut Text, (With<KillsUI>, Without<TimeUI>)>,
    game_state: Res<GameState>,
) {
    let minutes = (game_state.survived_seconds_this_night % 3600.) / 60.;
    let seconds = game_state.survived_seconds_this_night % 60.;
    **time = format!("Time: {:02}:{:02}", minutes.floor(), seconds.floor()).into();
    **kills = format!("Kills: {}", game_state.kills_this_night).into();
}
