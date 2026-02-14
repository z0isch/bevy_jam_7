use bevy::prelude::*;

use crate::{
    game::{GameState, GameStateMachine},
    theme::widget,
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameStateMachine::Shop), spawn_shop);
}

fn spawn_shop(mut commands: Commands) {
    commands.spawn((
        widget::ui_root("Shop"),
        GlobalZIndex(2),
        DespawnOnExit(GameStateMachine::Shop),
        children![
            widget::label("Shop"),
            widget::button("Next Dream", go_to_intro),
        ],
    ));
}

fn go_to_intro(
    _: On<Pointer<Click>>,
    mut state: ResMut<NextState<GameStateMachine>>,
    mut game_state: ResMut<GameState>,
) {
    game_state.dream_number += 1;
    state.set(GameStateMachine::Intro);
}
