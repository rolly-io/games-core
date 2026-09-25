//! Monte-Carlo RTP estimator for Blackjack.
//!
//! Measures the Return-To-Player of the *actual* engine
//! (`rolly_game_core::blackjack::replay_full`) under optimal **basic strategy**.
//! Blackjack RTP is not a closed-form constant like the other games: it depends
//! on the rule set (encoded in `blackjack.rs`) and on the player's strategy, so
//! we simulate many rounds and take `Σ win / Σ total_staked`.
//!
//! The engine is the single source of truth: this example only *decides* the
//! actions (basic strategy) and reads back the hands/points the engine reports;
//! all card dealing, splitting, doubling and payout stay in game-core.
//!
//! Rules (from `blackjack.rs`): 8 decks, fresh shuffle each round (no counting
//! edge), dealer stands on 17 incl. soft 17 (S17), blackjack pays 3:2 (2.5×),
//! double + one split + double-after-split, 10-card charlie, no surrender, no
//! resplit. Insurance is ALWAYS taken whenever it is offered (dealer shows an
//! Ace) — the classic "always insure" line, which is a losing side bet without
//! card counting and therefore lowers the RTP vs. basic strategy.
//!
//! Run:
//!   cargo run --release --example blackjack_rtp -- [rounds] [seed]
//!   # e.g. cargo run --release --example blackjack_rtp -- 10000000

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use rolly_game_core::blackjack::{
    deal_shoe, replay_full, BlackjackResult, ACTION_DOUBLE, ACTION_HIT, ACTION_INSURANCE_ACCEPT,
    ACTION_SPLIT, ACTION_STAND, MAX_CARDS,
};

/// Hard value of a rank code (Ace = 1 here; soft handling is separate).
fn card_value(code: u8) -> u32 {
    if code == 0 {
        1
    } else if code <= 8 {
        (code + 1) as u32
    } else {
        10
    }
}

/// Best hand total plus whether it is soft (an ace still counts as 11).
fn eval(cards: &[u8]) -> (u32, bool) {
    let mut total = 0u32;
    let mut aces = 0u32;
    for &c in cards {
        if c == 0 {
            aces += 1;
            total += 1;
        } else {
            total += card_value(c);
        }
    }
    if aces > 0 && total + 10 <= 21 {
        (total + 10, true)
    } else {
        (total, false)
    }
}

/// Dealer up-card value for strategy (Ace = 11).
fn dealer_up_val(code: u8) -> u32 {
    if code == 0 {
        11
    } else {
        card_value(code)
    }
}

/// Two cards of equal rank value (10/J/Q/K all count as a "10" pair).
fn is_pair(cards: &[u8]) -> bool {
    cards.len() == 2 && card_value(cards[0]) == card_value(cards[1])
}

/// Basic-strategy split decision (8-deck, S17, DAS).
fn should_split(cards: &[u8], up: u32) -> bool {
    if cards[0] == 0 && cards[1] == 0 {
        return true; // aces
    }
    match card_value(cards[0]) {
        10 => false,
        9 => matches!(up, 2..=6 | 8 | 9),
        8 => true,
        7 => up <= 7,
        6 => up <= 6,
        5 => false,
        4 => up == 5 || up == 6,
        3 => up <= 7,
        2 => up <= 7,
        _ => false,
    }
}

/// Basic-strategy action for a hard total. Returns HIT / STAND / DOUBLE.
fn hard_action(total: u32, up: u32) -> u8 {
    if total >= 17 {
        ACTION_STAND
    } else if total >= 13 {
        if up <= 6 {
            ACTION_STAND
        } else {
            ACTION_HIT
        }
    } else if total == 12 {
        if (4..=6).contains(&up) {
            ACTION_STAND
        } else {
            ACTION_HIT
        }
    } else if total == 11 {
        if up <= 10 {
            ACTION_DOUBLE
        } else {
            ACTION_HIT
        }
    } else if total == 10 {
        if up <= 9 {
            ACTION_DOUBLE
        } else {
            ACTION_HIT
        }
    } else if total == 9 {
        if (3..=6).contains(&up) {
            ACTION_DOUBLE
        } else {
            ACTION_HIT
        }
    } else {
        ACTION_HIT
    }
}

/// Basic-strategy action for a soft total (contains an ace as 11).
fn soft_action(total: u32, up: u32) -> u8 {
    match total {
        19 | 20 | 21 => ACTION_STAND,
        18 => {
            if (3..=6).contains(&up) {
                ACTION_DOUBLE
            } else if up == 2 || up == 7 || up == 8 {
                ACTION_STAND
            } else {
                ACTION_HIT
            }
        }
        17 => {
            if (3..=6).contains(&up) {
                ACTION_DOUBLE
            } else {
                ACTION_HIT
            }
        }
        15 | 16 => {
            if (4..=6).contains(&up) {
                ACTION_DOUBLE
            } else {
                ACTION_HIT
            }
        }
        13 | 14 => {
            if (5..=6).contains(&up) {
                ACTION_DOUBLE
            } else {
                ACTION_HIT
            }
        }
        _ => ACTION_HIT,
    }
}

/// Pick the next action for the active hand under basic strategy.
fn decide(cards: &[u8], up_code: u8, can_double: bool, can_split: bool) -> u8 {
    let up = dealer_up_val(up_code);
    if can_split && should_split(cards, up) {
        return ACTION_SPLIT;
    }
    let (total, soft) = eval(cards);
    let act = if soft {
        soft_action(total, up)
    } else {
        hard_action(total, up)
    };
    if act == ACTION_DOUBLE && !can_double {
        // Can't double (hand has >2 cards): the non-double fallback is STAND for
        // soft 18 (A,7 vs 3-6) and HIT for every other doubling total.
        return if soft && total == 18 {
            ACTION_STAND
        } else {
            ACTION_HIT
        };
    }
    act
}

/// Play one round with basic strategy, driving the engine action-by-action and
/// reading back the hands it reports. Returns the engine's authoritative result.
fn play_round(shoe: &[u8; MAX_CARDS], base: u64) -> BlackjackResult {
    let mut actions: Vec<u8> = Vec::new();
    let mut active = 0usize; // 0 = first hand, 1 = second (after split)
    let mut has_split = false;

    // Always take insurance when the dealer's up-card is an Ace (shoe[2] == 0):
    // insurance is only offered on a dealer Ace, and this is the "always insure"
    // line. The engine peeks the hole card on this action — if the dealer has a
    // natural, the round settles immediately with the 2:1 insurance payout.
    if shoe[2] == 0 {
        actions.push(ACTION_INSURANCE_ACCEPT);
    }

    loop {
        let res = replay_full(shoe, &actions, base);
        if res.is_finished {
            return res;
        }

        let cards = if active == 0 {
            res.first_cards.clone()
        } else {
            res.second_cards.clone()
        };

        // A hand that reached 21 / busted is closed; after a split move to the
        // second hand, otherwise the engine has already settled.
        let (pts, _) = eval(&cards);
        if pts >= 21 {
            if has_split && active == 0 {
                active = 1;
                continue;
            }
            return res; // guard: shouldn't happen (engine would be finished)
        }

        let up = res.dealer_cards[0];
        let can_split = !has_split && active == 0 && cards.len() == 2 && is_pair(&cards);
        let can_double = cards.len() == 2; // double-after-split allowed
        let act = decide(&cards, up, can_double, can_split);
        actions.push(act);

        match act {
            ACTION_SPLIT => has_split = true,
            ACTION_STAND | ACTION_DOUBLE => {
                if has_split && active == 0 {
                    active = 1; // first hand done → play the split hand
                }
            }
            _ => {}
        }

        if actions.len() > 40 {
            return res; // safety: never loop forever
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let rounds: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1_000_000);
    let seed: u64 = args
        .get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0xB1AC_C0FF_EE00_1234);
    let base: u64 = 1_000_000; // 1 USDT base stake

    // A round needing > MAX_CARDS cards makes the engine assert; those are
    // astronomically rare under basic strategy — silence the panic and skip them.
    std::panic::set_hook(Box::new(|_| {}));

    let mut rng = StdRng::seed_from_u64(seed);
    let mut total_win: u128 = 0;
    let mut total_staked: u128 = 0;
    let (mut counted, mut skipped) = (0u64, 0u64);
    let (mut wins, mut pushes, mut losses, mut naturals) = (0u64, 0u64, 0u64, 0u64);
    let mut insured = 0u64;

    for _ in 0..rounds {
        let swaps: [u64; MAX_CARDS] = core::array::from_fn(|_| rng.gen());
        let shoe = deal_shoe(&swaps);
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| play_round(&shoe, base)));
        match res {
            Ok(r) => {
                counted += 1;
                total_win += r.payout.win_amount as u128;
                total_staked += r.total_staked as u128;
                let net = r.payout.win_amount as i128 - r.total_staked as i128;
                if net > 0 {
                    wins += 1;
                } else if net == 0 {
                    pushes += 1;
                } else {
                    losses += 1;
                }
                if r.num_hands == 1 && r.first_points == 21 && r.first_cards.len() == 2 {
                    naturals += 1;
                }
                if r.insurance_taken {
                    insured += 1;
                }
            }
            Err(_) => skipped += 1,
        }
    }

    let _ = std::panic::take_hook();

    let rtp = total_win as f64 / total_staked as f64;
    let pct = |x: u64| 100.0 * x as f64 / counted.max(1) as f64;

    println!("Blackjack RTP — Monte-Carlo over {counted} rounds (seed {seed})");
    println!("  rules: 8 decks · S17 · BJ 3:2 · DAS · 1 split · charlie · no surrender · basic strategy · ALWAYS insure");
    println!("  RTP         : {:.4}%", rtp * 100.0);
    println!("  house edge  : {:.4}%", (1.0 - rtp) * 100.0);
    println!(
        "  total win   : {} · total staked: {} (avg staked {:.4}× base)",
        total_win,
        total_staked,
        total_staked as f64 / (counted.max(1) as f64 * base as f64)
    );
    println!(
        "  outcomes    : win {:.2}% · push {:.2}% · loss {:.2}% · player BJ {:.2}%",
        pct(wins),
        pct(pushes),
        pct(losses),
        pct(naturals)
    );
    println!("  insured     : {:.2}% of rounds (dealer Ace up)", pct(insured));
    if skipped > 0 {
        println!("  skipped     : {skipped} rounds (needed > {MAX_CARDS} cards)");
    }
}
