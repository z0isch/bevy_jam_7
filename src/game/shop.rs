use bevy::prelude::*;

use crate::{
    game::{GameState, GameStateMachine},
    screens::Screen,
    theme::widget,
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameStateMachine::Shop), spawn_shop);
}

fn spawn_shop(mut commands: Commands, game_state: Res<GameState>) {
    let minutes = (game_state.survived_seconds_this_night % 3600.) / 60.;
    let seconds = game_state.survived_seconds_this_night % 60.;
    commands.spawn((
        widget::ui_root("Shop"),
        GlobalZIndex(1),
        DespawnOnExit(GameStateMachine::Shop),
        DespawnOnExit(Screen::Gameplay),
        children![
            widget::label(format!("Night #{}", game_state.night_number)),
            widget::label(format!("Kills: {}", game_state.kills_this_night)),
            widget::label(format!(
                "Survived: {:02}:{:02}",
                minutes.floor(),
                seconds.floor()
            )),
            widget::button("Next", go_to_intro),
        ],
    ));
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
