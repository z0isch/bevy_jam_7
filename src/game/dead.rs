use bevy::prelude::*;

use crate::{
    game::{GameState, GameStateMachine},
    screens::Screen,
    theme::widget,
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameStateMachine::Dead), spawn_dead);
}

fn spawn_dead(mut commands: Commands, game_state: ResMut<GameState>) {
    let minutes = (game_state.survived_seconds_this_night % 3600.) / 60.;
    let seconds = game_state.survived_seconds_this_night % 60.;
    commands.spawn((
        widget::ui_root("Dead"),
        GlobalZIndex(1),
        DespawnOnExit(GameStateMachine::Dead),
        DespawnOnExit(Screen::Gameplay),
        children![
            widget::header("You didn't survive!"),
            widget::label("Can you last until sunrise (2.5 minutes)?"),
            widget::label(""),
            widget::label(format!("Kills: {}", game_state.kills_this_night)),
            widget::label(format!(
                "Survived for: {:02}:{:02}",
                minutes.floor(),
                seconds.floor()
            )),
            widget::label(""),
            widget::button("Shop", go_to_shop),
        ],
    ));
}

fn go_to_shop(_: On<Pointer<Click>>, mut state: ResMut<NextState<GameStateMachine>>) {
    state.set(GameStateMachine::Shop);
}
