use std::collections::BTreeMap;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;

use crate::big_num::BigDollar;
use crate::game::{Game, GameState};
use crate::pane_module::ModuleId;
use crate::text_sources::TextSource;

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
    AutoLetterCount,
}

pub struct Upgrade {
    // List of costs per level, the length of the vector is the max level
    pub costs: Vec<BigDollar>,
    pub infinite: bool,
    pub name: &'static str,
    pub description: &'static str,
    pub upgrade_unlock_condition: fn(&GameState) -> bool,
    pub on_buy: fn(&mut Game),
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
                on_buy: |game| game.game_state.base_letter_value += BigDollar::from(0.001),
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
                    game.game_state.total_money_earned += BigDollar::from(0) - game.game_state.money;
                    game.game_state.money = BigDollar::from(0);
                    game.game_state.trust_level -= 1;
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
                on_buy: |game| game.game_state.streaks_unlocked = true,
            },
        ),
        (
            UpgradeId::CapitalLetterBonus,
            Upgrade {
                costs: vec![BigDollar::from(5.0)],
                infinite: false,
                name: "Discover the shift key",
                description: "Capital letters are worth 10x",
                upgrade_unlock_condition: |game| game.high_water_money >= BigDollar::from(1.0),
                on_buy: |game| {
                    game.game_state.capital_letter_bonus_unlocked = true;
                    game.game_state.text_pane.current_text = TextSource::IntroCapital;
                },
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
                on_buy: |game| game.game_state.automation_unlocked = true,
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
                on_buy: |game| game.game_state.seniority_level += 1,
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
                on_buy: |game| game.game_state.graphs_unlocked = true,
            },
        ),
        (
            UpgradeId::LetterCompression,
            Upgrade {
                costs: vec![BigDollar::from(100.0)],
                infinite: false,
                name: "Letter compression",
                description: "Lowercase letters are compressed to uppercase letters",
                upgrade_unlock_condition: |game| game.total_money_earned >= BigDollar::from(50.0) && game.capital_letter_bonus_unlocked,
                on_buy: |game| game.game_state.auto_queue.letter_compression_unlocked = true,
            },
        ),
        (
            UpgradeId::AutoLetterCount,
            Upgrade {
                costs: (0..200)
                    .map(|i| BigDollar::from(0.2 * (i as f64) * (i as f64) + 1000.0))
                    .collect(),
                infinite: false,
                name: "Auto letter count",
                description: "The letter machine automatically counts letters for you",
                upgrade_unlock_condition: |game| game.total_money_earned >= BigDollar::from(50.0) && game.automation_unlocked,
                on_buy: |game| {
                    game.game_state.auto_queue.auto_typer_selector.max_auto_typers += 1;
                    game.add_floating_pane(ModuleId::AutoTyperSelector);
                },
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
                on_buy: |game| game.game_state.disable_penalty = true,
            },
        ),
    ])
});

pub fn get_upgrades() -> &'static BTreeMap<UpgradeId, Upgrade> {
    &UPGRADES
}
