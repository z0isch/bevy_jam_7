use bevy::prelude::*;
use bevy_rand::prelude::*;
use rand::Rng;

use crate::{
    game::{GameState, GameStateMachine},
    quotes::QUOTES,
    screens::Screen,
    theme::widget,
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameStateMachine::Intro), spawn_intro);
}

fn spawn_intro(
    mut commands: Commands,
    game_state: Res<GameState>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
) {
    let random_quote_idx = rng.random_range(0..18);
    let [quote, author] = QUOTES[random_quote_idx];
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
