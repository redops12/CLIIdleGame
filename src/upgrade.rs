use std::collections::BTreeMap;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;

use crate::big_num::BigDollar;
use crate::game::GameState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter, PartialOrd, Ord, Serialize, Deserialize)]
pub enum UpgradeId {
    IncreaseCorrectIncrement,
    ResetMoneyForTrust,
    UnlockStreak,
    CapitalLetterBonus,
    UnlockAutomation,
    Seniority,
    Graphs,
    LetterCompression,
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

static UPGRADES: LazyLock<BTreeMap<UpgradeId, Upgrade>> = LazyLock::new(|| {
    BTreeMap::from([
        (
            UpgradeId::IncreaseCorrectIncrement,
            Upgrade {
                costs: (0..=49)
                    .map(|i| BigDollar::from(0.05 * 1.2_f64.powi(i)))
                    .collect(),
                infinite: false,
                name: "Writing Skill",
                description: "Add a 10th more cent per correct letter",
                upgrade_unlock_condition: |_game| true,
                on_buy: |game| game.base_letter_value += BigDollar::from(0.001),
            },
        ),
        (
            UpgradeId::ResetMoneyForTrust,
            Upgrade {
                costs: vec![BigDollar::from(0)],
                infinite: true,
                name: "Beg for help",
                description: "Lose trust to get back to $0",
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
                costs: vec![BigDollar::from(1.0)],
                infinite: false,
                name: "Unlock Trust",
                description: "Profit is multiplied by (1.15)^trust",
                upgrade_unlock_condition: |game| game.high_water_money >= BigDollar::from(0.15),
                on_buy: |game| game.streaks_unlocked = true,
            },
        ),
        (
            UpgradeId::CapitalLetterBonus,
            Upgrade {
                costs: vec![BigDollar::from(5.0)],
                infinite: false,
                name: "Discover the shift key",
                description: "Capital letters are worth 5x",
                upgrade_unlock_condition: |game| game.high_water_money >= BigDollar::from(1.0),
                on_buy: |game| game.capital_letter_bonus_unlocked = true,
            },
        ),
        (
            UpgradeId::UnlockAutomation,
            Upgrade {
                costs: vec![BigDollar::from(10.0)],
                infinite: false,
                name: "Let the robots write",
                description: "Unlock the letter machine",
                upgrade_unlock_condition: |game| game.high_water_money >= BigDollar::from(1.0),
                on_buy: |game| game.automation_unlocked = true,
            },
        ),
        (
            UpgradeId::Seniority,
            Upgrade {
                costs: vec![BigDollar::from(20.0), BigDollar::from(100.0), BigDollar::from(500.0), BigDollar::from(2000.0), BigDollar::from(10000.0)],
                infinite: false,
                name: "Seniority",
                description: "For each level of seniority, 20% lower correctness requirement to gain trust",
                upgrade_unlock_condition: |game| game.high_water_money >= BigDollar::from(5.0),
                on_buy: |game| game.seniority_level += 1,
            },
        ),
        (
            UpgradeId::Graphs,
            Upgrade {
                costs: vec![BigDollar::from(50.0)],
                infinite: false,
                name: "Graphs",
                description: "Visualize your progress",
                upgrade_unlock_condition: |game| game.high_water_money >= BigDollar::from(10.0),
                on_buy: |game| game.graphs_unlocked = true,
            },
        ),
        (
            UpgradeId::LetterCompression,
            Upgrade {
                costs: vec![BigDollar::from(100.0)],
                infinite: false,
                name: "Letter compression",
                description: "Lowercase letters are compressed to uppercase letters",
                upgrade_unlock_condition: |game| game.total_money_earned >= BigDollar::from(10.0),
                on_buy: |game| game.letter_compression_unlocked = true,
            },
        ),
        (
            UpgradeId::DisablePenalty,
            Upgrade {
                costs: vec![BigDollar::from(1e7_f64)],
                infinite: false,
                name: "Pay off editors",
                description: "Mistakes no longer cost money",
                upgrade_unlock_condition: |game| game.total_money_earned >= BigDollar::from(1e6_f64),
                on_buy: |game| game.disable_penalty = true,
            },
        ),
    ])
});

pub fn get_upgrades() -> &'static BTreeMap<UpgradeId, Upgrade> {
    &UPGRADES
}
