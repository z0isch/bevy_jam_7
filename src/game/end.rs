use bevy::prelude::*;

use crate::{game::GameStateMachine, screens::Screen, theme::widget};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameStateMachine::End), spawn_end);
}

fn spawn_end(mut commands: Commands) {
    commands.spawn((
        widget::ui_root("END"),
        GlobalZIndex(1),
        DespawnOnExit(GameStateMachine::End),
        DespawnOnExit(Screen::Gameplay),
        children![
            widget::header("The End"),
            widget::label("Even the darkest night will end and the sun will rise."),
            widget::label("- Victor Hugo, Les Miserables"),
            widget::header(""),
            widget::label("Thanks for playing!")
        ],
    ));
}
