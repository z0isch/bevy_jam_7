use bevy::prelude::*;

use crate::{
    game::{GameState, GameStateMachine},
    screens::Screen,
    theme::widget,
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameStateMachine::Intro), spawn_intro);
}

fn spawn_intro(mut commands: Commands, mut game_state: ResMut<GameState>) {
    let quotes_len = game_state.quotes.len();
    let (quote, author) = game_state.quotes[game_state.current_quote_index].clone();
    game_state.current_quote_index = (game_state.current_quote_index + 1) % quotes_len;

    commands.spawn((
        widget::ui_root("Main Menu"),
        GlobalZIndex(1),
        DespawnOnExit(GameStateMachine::Intro),
        DespawnOnExit(Screen::Gameplay),
        children![
            widget::header(format!("Night #{:}", game_state.night_number)),
            widget::label(quote),
            widget::label(author),
            widget::button("Start", go_to_level),
        ],
    ));
}

fn go_to_level(_: On<Pointer<Click>>, mut state: ResMut<NextState<GameStateMachine>>) {
    state.set(GameStateMachine::Level);
}
