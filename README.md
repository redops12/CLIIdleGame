# CLI Idle Game

A typing-based idle / incremental game for the terminal, written in Rust.

You type passages of text to earn money. Correct characters pay; mistakes hurt (until you buy your way out of that). Spend earnings on upgrades that raise letter value, unlock trust multipliers, and eventually automate typing entirely — so progress keeps going while you watch the letter machine sort and fire characters into the queue.

## Concept

The fantasy is office-drone typing that slowly becomes something else. You start with simple lines and manual keypresses. Upgrades open capital letters, trust streaks, graphs, and a robot letter machine. Later systems lean toward volume and automation rather than just higher per-character payouts. Longer-term ideas (diverging stories, marketing loops, moral choices about what you type) live in `game_plans.md`.

## Requirements

- Rust toolchain (edition 2024)
- A terminal that supports mouse capture (optional; keyboard works either way)

## Run

```bash
cargo run --release
```

On startup you can load a save or start fresh. On quit you are prompted to save.

### Useful flags

| Flag | Effect |
|------|--------|
| `--save <path>` | Load a specific save file and skip the startup prompt |
| `--prompt` | Force save/load prompts even in test mode |
| `--test <module>` | Open a single UI module for layout debugging |

## How to play

1. Type the highlighted line in the text pane. Matching characters earn money.
2. Buy upgrades with keys `q w e r t a s d f g` (or click them) when you can afford them.
3. Switch focus between panes with the arrow keys / window navigation.
4. Once automation unlocks, feed the letter machine and let queues type for you.
5. Watch the money bar (and later the profit graph) to decide what to buy next.

## Implementation

Terminal UI is built with **ratatui** + **crossterm**. Input is polled on a background thread and sent into the main loop over a **crossbeam** channel. The game ticks at ~60 FPS: update state, draw panes, sleep the remainder of the frame.

### Layout

Core loop and wiring live in `src/main.rs`. Game state and input handling are in `src/game.rs`. The screen is composed of modular panes (`src/pane_module.rs`, `src/ui.rs`):

- **Text pane** — reference lines vs typed input, correctness coloring
- **Upgrade pane** — purchasable upgrades gated by money / unlock conditions
- **Money bar** — current balance via `BigDollar`
- **Auto queue / letter machine** — sorting, queues, and auto-typers
- **Graph pane** — rolling profit visualization (unlockable)

### Progression

Upgrades are defined in `src/upgrade.rs` as a static map of costs, unlock predicates, and `on_buy` side effects. Examples: raise base letter value, unlock trust multipliers, capital-letter bonuses, automation, seniority (easier trust gain), letter compression, faster sorting, and disabling mistake penalties.

Money uses `BigDollar` (`src/big_num.rs`) so balances can grow into abbreviated large numbers without leaving floating-point display land.

### Content & persistence

Passages are plain text under `assets/` and selected via `TextSource` (`src/text_sources.rs`). Game state serializes with **serde** / **serde_json** (`src/save.rs`) so runs can be saved and resumed.

## Project layout

```
src/
  main.rs              # entry, input thread, frame loop
  game.rs              # Game / GameState, input routing
  upgrade.rs           # upgrade definitions
  text_pane.rs         # typing UI
  auto_queue.rs        # letter machine & queues
  ui.rs                # pane layout
  save.rs              # load / save prompts
  big_num.rs           # money type
  ...
assets/                # typed passages (intro, bartleby, wasteland, …)
game_plans.md          # backlog and design notes
```

## License

Personal / WIP project — no license declared yet.
