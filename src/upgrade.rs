use crossterm::event::KeyCode;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::BigNum::BigDollar;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter)]
pub enum Upgrade {
    IncreaseCorrectIncrement,
    DecreaseIncorrectPenalty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter)]
pub enum GameValue {
    Increment,
    Penalty,
}

pub fn upgrade_starting_cost(kind: Upgrade) -> BigDollar {
    match kind {
        Upgrade::IncreaseCorrectIncrement => BigDollar::from(100),
        Upgrade::DecreaseIncorrectPenalty => BigDollar::from(1000),
    }
}

pub fn upgrade_multiplier(kind: Upgrade) -> f32 {
    match kind {
        Upgrade::IncreaseCorrectIncrement => 1.2,
        Upgrade::DecreaseIncorrectPenalty => 10.0,
    }
}

pub fn upgrade_buttons(kind: Upgrade) -> KeyCode {
    match kind {
        Upgrade::IncreaseCorrectIncrement => KeyCode::Char('q'),
        Upgrade::DecreaseIncorrectPenalty => KeyCode::Char('w'),
    }
}

pub fn buttons_to_upgrades() -> std::collections::HashMap<KeyCode, Upgrade> {
    Upgrade::iter()
        .map(|kind| (upgrade_buttons(kind), kind))
        .collect()
}

pub fn button_to_upgrade(button: KeyCode) -> Option<Upgrade> {
    buttons_to_upgrades().get(&button).copied()
}

pub fn upgrade_descriptions(kind: Upgrade) -> &'static str {
    match kind {
        Upgrade::IncreaseCorrectIncrement => "Increase reward for correct characters",
        Upgrade::DecreaseIncorrectPenalty => "Decrease penalty for incorrect characters",
    }
}

pub fn upgrade_value_key(kind: Upgrade) -> GameValue {
    match kind {
        Upgrade::IncreaseCorrectIncrement => GameValue::Increment,
        Upgrade::DecreaseIncorrectPenalty => GameValue::Penalty,
    }
}

pub fn upgrade_value_change(kind: Upgrade) -> BigDollar {
    match kind {
        Upgrade::IncreaseCorrectIncrement => BigDollar::from(1),
        Upgrade::DecreaseIncorrectPenalty => BigDollar::from(-1),
    }
}

pub fn game_value_start(value: GameValue) -> BigDollar {
    match value {
        GameValue::Increment => BigDollar::from(1),
        GameValue::Penalty => BigDollar::from(5),
    }
}

pub fn initial_upgrade_costs() -> std::collections::HashMap<Upgrade, BigDollar> {
    Upgrade::iter()
        .map(|kind| (kind, upgrade_starting_cost(kind)))
        .collect()
}

pub fn initial_game_values() -> std::collections::HashMap<GameValue, BigDollar> {
    GameValue::iter()
        .map(|value| (value, game_value_start(value)))
        .collect()
}

pub fn value_to_string(value: GameValue) -> String {
    match value {
        GameValue::Increment => "Increment".to_string(),
        GameValue::Penalty => "Penalty".to_string(),
    }
}
