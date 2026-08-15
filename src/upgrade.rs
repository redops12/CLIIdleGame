use strum_macros::EnumIter;
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::BigNum::BigDollar;
use crate::game::GameState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter)]
pub enum UpgradeId {
    IncreaseCorrectIncrement,
    ResetMoneyForTrust,
    UnlockStreak,
    DisablePenalty,
}

pub struct Upgrade {
    // List of costs per level, the length of the vector is the max level
    pub costs: Vec<BigDollar>,
    pub infinite: bool,
    pub name: &'static str,
    pub description: &'static str,
    pub upgrade_unlock_condition: fn(&GameState) -> bool,
    pub on_buy: fn(&mut GameState),
}

static UPGRADES: LazyLock<HashMap<UpgradeId, Upgrade>> = LazyLock::new(|| {
    HashMap::from([
        (
            UpgradeId::IncreaseCorrectIncrement,
            Upgrade {
                costs: (0..=49)
                    .map(|i| BigDollar::from(50.0 * 1.1_f64.powi(i)))
                    .collect(),
                infinite: false,
                name: "Valuable letters",
                description: "Increase your letter writing skill",
                upgrade_unlock_condition: |_game| true,
                on_buy: |game| game.base_letter_value += BigDollar::from(1),
            },
        ),
        (
            UpgradeId::ResetMoneyForTrust,
            Upgrade {
                costs: vec![BigDollar::from(0)],
                infinite: true,
                name: "Beg for help",
                description: "Lose trust to reset your money",
                upgrade_unlock_condition: |game| game.money < BigDollar::from(0),
                on_buy: |game| {
                    game.total_money_earned += BigDollar::from(0) - game.money;
                    game.money = BigDollar::from(0);
                    game.trust_level -= 1;
                },
            },
        ),
        (
            UpgradeId::UnlockStreak,
            Upgrade {
                costs: vec![BigDollar::from(150)],
                infinite: false,
                name: "Unlock Trust",
                description: "More correct letters, more trust, more value",
                upgrade_unlock_condition: |game| game.total_money_earned >= BigDollar::from(50),
                on_buy: |game| game.streaks_unlocked = true,
            },
        ),
        (
            UpgradeId::DisablePenalty,
            Upgrade {
                costs: vec![BigDollar::from(10000)],
                infinite: false,
                name: "Pay off editors",
                description: "Income is no longer affected by mistakes",
                upgrade_unlock_condition: |game| game.total_money_earned >= BigDollar::from(1000),
                on_buy: |game| game.disable_penalty = true,
            },
        ),
    ])
});

pub fn get_upgrades() -> &'static HashMap<UpgradeId, Upgrade> {
    &UPGRADES
}
