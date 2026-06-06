use rolly_game_core::dice::*;
use rolly_game_core::shared::*;

fn random_with_roll(desired_roll: u32) -> [u64; 4] {
    [desired_roll as u64, 0, 0, 0]
}

// ─── Under mode ───────────────────────────────────────────────

#[test]
fn under_win_boundary() {
    assert!(is_win(DiceMode::Under, [500, 0], 499));
}

#[test]
fn under_lose_boundary() {
    assert!(!is_win(DiceMode::Under, [500, 0], 500));
}

#[test]
fn under_lose_above() {
    assert!(!is_win(DiceMode::Under, [500, 0], 501));
}

// ─── Over mode ────────────────────────────────────────────────

#[test]
fn over_win_boundary() {
    assert!(is_win(DiceMode::Over, [500, 0], 501));
}

#[test]
fn over_lose_boundary() {
    assert!(!is_win(DiceMode::Over, [500, 0], 500));
}

#[test]
fn over_lose_below() {
    assert!(!is_win(DiceMode::Over, [500, 0], 499));
}

// ─── In mode ──────────────────────────────────────────────────

#[test]
fn in_win_left_edge() {
    assert!(is_win(DiceMode::In, [250, 750], 250));
}

#[test]
fn in_win_right_edge() {
    assert!(is_win(DiceMode::In, [250, 750], 750));
}

#[test]
fn in_win_middle() {
    assert!(is_win(DiceMode::In, [250, 750], 500));
}

#[test]
fn in_lose_left() {
    assert!(!is_win(DiceMode::In, [250, 750], 249));
}

#[test]
fn in_lose_right() {
    assert!(!is_win(DiceMode::In, [250, 750], 751));
}

// ─── Out mode ─────────────────────────────────────────────────

#[test]
fn out_win_left() {
    assert!(is_win(DiceMode::Out, [250, 750], 249));
}

#[test]
fn out_win_right() {
    assert!(is_win(DiceMode::Out, [250, 750], 751));
}

#[test]
fn out_lose_inside() {
    assert!(!is_win(DiceMode::Out, [250, 750], 500));
}

#[test]
fn out_lose_left_edge() {
    assert!(!is_win(DiceMode::Out, [250, 750], 250));
}

#[test]
fn out_lose_right_edge() {
    assert!(!is_win(DiceMode::Out, [250, 750], 750));
}

// ─── Edge cases: roll=0, roll=999 ─────────────────────────────

#[test]
fn roll_zero_under_min_range() {
    assert!(is_win(DiceMode::Under, [1, 0], 0));
}

#[test]
fn roll_zero_over_zero_loses() {
    assert!(!is_win(DiceMode::Over, [0, 0], 0));
}

#[test]
fn roll_max_over_998() {
    assert!(is_win(DiceMode::Over, [998, 0], 999));
}

#[test]
fn roll_max_under_999_loses() {
    assert!(!is_win(DiceMode::Under, [999, 0], 999));
}

// ─── win_numbers ──────────────────────────────────────────────

#[test]
fn win_numbers_under() {
    assert_eq!(win_numbers(DiceMode::Under, [500, 0]), 500);
}

#[test]
fn win_numbers_over() {
    assert_eq!(win_numbers(DiceMode::Over, [500, 0]), 499);
}

#[test]
fn win_numbers_in() {
    assert_eq!(win_numbers(DiceMode::In, [250, 750]), 501);
}

#[test]
fn win_numbers_out() {
    assert_eq!(win_numbers(DiceMode::Out, [250, 750]), 499);
}

// ─── Multiplier precision ─────────────────────────────────────

#[test]
fn multi_exact_500() {
    assert_eq!(multiplier(500), 19800); // 1.9800x
}

#[test]
fn multi_min_range() {
    assert_eq!(multiplier(1), 9_900_000); // 990.0000x
}

#[test]
fn multi_max_range() {
    // 9_900_000 / 950 = 10421 (integer floor)
    assert_eq!(multiplier(950), 10421); // 1.0421x
}

#[test]
fn multi_floor_division() {
    // wn=7: 9900000/7 = 1414285.714... → floor → 1414285
    assert_eq!(multiplier(7), 1_414_285);
}

#[test]
fn multi_wn_3() {
    // 9900000 / 3 = 3300000 exactly
    assert_eq!(multiplier(3), 3_300_000);
}

// ─── max_win cap ──────────────────────────────────────────────

#[test]
fn max_win_cap_applied() {
    let bet = 700 * USDT_DECIMALS; // 700 USDT
    let random = random_with_roll(0); // roll=0
    // Under with prediction=[1] → wn=1, multi=990x → raw = 700*990 = 693000 USDT
    let payout = compute_payout(&random, bet, DiceMode::Under, [1, 0]);
    assert!(payout.is_win);
    assert_eq!(payout.win_amount, MAX_WIN); // capped at 10000 USDT
}

#[test]
fn below_max_win_not_capped() {
    let bet = 10 * USDT_DECIMALS; // 10 USDT
    let random = random_with_roll(0);
    let payout = compute_payout(&random, bet, DiceMode::Under, [1, 0]);
    assert!(payout.is_win);
    // 10 USDT * 990x = 9900 USDT < 10000 cap
    let expected = (10u64 * USDT_DECIMALS as u64 * 9_900_000) / 10_000;
    assert_eq!(payout.win_amount, expected);
    assert!(payout.win_amount < MAX_WIN);
}

// ─── Zero win on loss ─────────────────────────────────────────

#[test]
fn zero_win_on_lose() {
    let random = random_with_roll(500);
    let payout = compute_payout(&random, 100 * USDT_DECIMALS, DiceMode::Under, [500, 0]);
    assert!(!payout.is_win);
    assert_eq!(payout.win_amount, 0);
}

// ─── u128 no overflow ─────────────────────────────────────────

#[test]
fn u128_no_overflow_max_scenario() {
    // max_bet=700 USDT, max_multi=990x (wn=1)
    let bet = 700 * USDT_DECIMALS;
    let multi = multiplier(1); // 9_900_000
    let raw = (bet as u128 * multi as u128) / 10_000;
    // 700_000_000 * 9_900_000 / 10_000 = 693_000_000_000 — fits u64 easily
    assert!(raw < u64::MAX as u128);
    assert_eq!(raw, 693_000_000_000u128);
}

// ─── Full payout integration ──────────────────────────────────

#[test]
fn compute_payout_under_win() {
    let random = random_with_roll(250);
    let payout = compute_payout(&random, 50 * USDT_DECIMALS, DiceMode::Under, [500, 0]);
    assert!(payout.is_win);
    assert_eq!(payout.roll_number, 250);
    assert_eq!(payout.multiplier, 19800);
    // 50_000_000 * 19800 / 10000 = 99_000_000 (99 USDT)
    assert_eq!(payout.win_amount, 99_000_000);
}

#[test]
fn compute_payout_over_lose() {
    let random = random_with_roll(400);
    let payout = compute_payout(&random, 50 * USDT_DECIMALS, DiceMode::Over, [500, 0]);
    assert!(!payout.is_win);
    assert_eq!(payout.roll_number, 400);
    assert_eq!(payout.win_amount, 0);
}

#[test]
fn compute_payout_in_win() {
    let random = random_with_roll(500);
    let payout = compute_payout(&random, 100 * USDT_DECIMALS, DiceMode::In, [250, 750]);
    assert!(payout.is_win);
    // wn = 750 - 250 + 1 = 501, multi = 9900000/501 = 19760
    assert_eq!(payout.multiplier, 19760);
    // 100_000_000 * 19760 / 10000 = 197_600_000
    assert_eq!(payout.win_amount, 197_600_000);
}

#[test]
fn compute_payout_out_win() {
    let random = random_with_roll(800);
    let payout = compute_payout(&random, 20 * USDT_DECIMALS, DiceMode::Out, [250, 750]);
    assert!(payout.is_win);
    // wn = 1000 - 501 = 499, multi = 9900000/499 = 19839
    assert_eq!(payout.multiplier, 19839);
    // 20_000_000 * 19839 / 10000 = 39_678_000
    assert_eq!(payout.win_amount, 39_678_000);
}

// ─── roll_number extraction ───────────────────────────────────

#[test]
fn roll_number_wraps_correctly() {
    let random = [1999u64, 0, 0, 0];
    assert_eq!(roll_number_from_random(&random), 999);
}

#[test]
fn roll_number_large_value() {
    let random = [18_446_744_069_414_584_320u64, 0, 0, 0]; // p - 1
    assert_eq!(roll_number_from_random(&random), (18_446_744_069_414_584_320u64 % 1000) as u32);
}

// ─── DiceMode::from_u8 ───────────────────────────────────────

#[test]
fn dice_mode_from_u8() {
    assert_eq!(DiceMode::from_u8(0), Some(DiceMode::Under));
    assert_eq!(DiceMode::from_u8(1), Some(DiceMode::Over));
    assert_eq!(DiceMode::from_u8(2), Some(DiceMode::In));
    assert_eq!(DiceMode::from_u8(3), Some(DiceMode::Out));
    assert_eq!(DiceMode::from_u8(4), None);
    assert_eq!(DiceMode::from_u8(255), None);
}

// ─── RTP simulation ──────────────────────────────────────────

mod rtp_simulation {
    use super::*;
    use rand::prelude::*;
    use rand::rngs::StdRng;

    const DEFAULT_ITERATIONS: u64 = 1_000_000_000;

    fn iteration_count() -> u64 {
        std::env::var("RTP_ITERATIONS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_ITERATIONS)
    }

    fn random_mode(rng: &mut StdRng) -> DiceMode {
        match rng.gen_range(0u8..4) {
            0 => DiceMode::Under,
            1 => DiceMode::Over,
            2 => DiceMode::In,
            _ => DiceMode::Out,
        }
    }

    fn random_valid_prediction(mode: DiceMode, rng: &mut StdRng) -> [u32; 2] {
        match mode {
            DiceMode::Under => {
                [rng.gen_range(MIN_RANGE..=MAX_RANGE), 0]
            }
            DiceMode::Over => {
                let lo = TOTAL_RANGE - 1 - MAX_RANGE; // 49
                let hi = TOTAL_RANGE - 1 - MIN_RANGE; // 998
                [rng.gen_range(lo..=hi), 0]
            }
            DiceMode::In => {
                let wn = rng.gen_range(MIN_RANGE..=MAX_RANGE);
                let max_lo = TOTAL_RANGE - wn;
                let lo = rng.gen_range(0..=max_lo);
                [lo, lo + wn - 1]
            }
            DiceMode::Out => {
                let wn = rng.gen_range(MIN_RANGE..=MAX_RANGE);
                let inner = TOTAL_RANGE - wn;
                let max_lo = TOTAL_RANGE - inner;
                let lo = rng.gen_range(0..=max_lo);
                [lo, lo + inner - 1]
            }
        }
    }

    fn random_bet_atomic(rng: &mut StdRng) -> u64 {
        rng.gen_range(10_000..=700_000_000) // 0.01 USDT .. 700 USDT
    }

    fn random_poseidon2_output(rng: &mut StdRng) -> [u64; 4] {
        [rng.gen(), rng.gen(), rng.gen(), rng.gen()]
    }

    fn uncapped_win(bet: u64, multi: u64, won: bool) -> u128 {
        if won {
            (bet as u128 * multi as u128) / 10_000
        } else {
            0
        }
    }

    struct ModeStats {
        total_bet: u128,
        total_win: u128,
        count: u64,
        wins: u64,
    }

    impl ModeStats {
        fn new() -> Self {
            Self { total_bet: 0, total_win: 0, count: 0, wins: 0 }
        }

        fn rtp_pct(&self) -> f64 {
            if self.total_bet == 0 {
                return 0.0;
            }
            (self.total_win as f64 / self.total_bet as f64) * 100.0
        }
    }

    const THEORETICAL_RTP: f64 = 98.9978;

    #[test]
    #[ignore]
    fn rtp_simulation_dice() {
        let n = iteration_count();
        let mut rng = StdRng::seed_from_u64(42);

        let mut per_mode = [
            ModeStats::new(),
            ModeStats::new(),
            ModeStats::new(),
            ModeStats::new(),
        ];
        let mut total_bet: u128 = 0;
        let mut total_win: u128 = 0;

        for _ in 0..n {
            let mode = random_mode(&mut rng);
            let pred = random_valid_prediction(mode, &mut rng);
            let bet = random_bet_atomic(&mut rng);
            let random = random_poseidon2_output(&mut rng);

            let roll = roll_number_from_random(&random);
            let won = is_win(mode, pred, roll);
            let wn = win_numbers(mode, pred);
            let multi = multiplier(wn);
            let win = uncapped_win(bet, multi, won);

            total_bet += bet as u128;
            total_win += win;

            let s = &mut per_mode[mode as usize];
            s.total_bet += bet as u128;
            s.total_win += win;
            s.count += 1;
            if won {
                s.wins += 1;
            }
        }

        let rtp = (total_win as f64 / total_bet as f64) * 100.0;

        println!();
        println!("=== DICE RTP SIMULATION ===");
        println!("Iterations:  {n}");
        println!(
            "Total bet:   {total_bet} atomic ({:.2} USDT)",
            total_bet as f64 / 1e6
        );
        println!(
            "Total win:   {total_win} atomic ({:.2} USDT)",
            total_win as f64 / 1e6
        );
        println!("RTP:         {rtp:.6}%");
        println!("House edge:  {:.6}%", 100.0 - rtp);
        println!("Theoretical: {THEORETICAL_RTP:.4}%");
        println!();

        let names = ["Under", "Over ", "In   ", "Out  "];
        for (i, name) in names.iter().enumerate() {
            let s = &per_mode[i];
            println!(
                "  {name}  n={:>12}  wins={:>12}  bet={:>22}  win={:>22}  RTP={:.6}%  edge={:.6}%",
                s.count,
                s.wins,
                s.total_bet,
                s.total_win,
                s.rtp_pct(),
                100.0 - s.rtp_pct(),
            );
        }
        println!();

        assert!(
            rtp >= 98.90,
            "Overall RTP too low: {rtp:.6}% — payout math is broken"
        );
        assert!(
            rtp <= 99.10,
            "Overall RTP too high: {rtp:.6}% — user has edge!"
        );

        for (i, name) in ["Under", "Over", "In", "Out"].iter().enumerate() {
            let mode_rtp = per_mode[i].rtp_pct();
            assert!(
                mode_rtp >= 98.50,
                "{name} RTP too low: {mode_rtp:.6}%"
            );
            assert!(
                mode_rtp <= 99.50,
                "{name} RTP too high: {mode_rtp:.6}%"
            );
        }
    }

    #[test]
    #[ignore]
    fn rtp_per_mode_dice() {
        let per_mode_n = iteration_count() / 4;
        let mut rng = StdRng::seed_from_u64(123);

        println!();
        for mode in [DiceMode::Under, DiceMode::Over, DiceMode::In, DiceMode::Out] {
            let mut total_bet: u128 = 0;
            let mut total_win: u128 = 0;
            let mut wins: u64 = 0;

            for _ in 0..per_mode_n {
                let pred = random_valid_prediction(mode, &mut rng);
                let bet = random_bet_atomic(&mut rng);
                let random = random_poseidon2_output(&mut rng);

                let roll = roll_number_from_random(&random);
                let won = is_win(mode, pred, roll);
                let wn = win_numbers(mode, pred);
                let multi = multiplier(wn);
                let win = uncapped_win(bet, multi, won);

                total_bet += bet as u128;
                total_win += win;
                if won {
                    wins += 1;
                }
            }

            let rtp = (total_win as f64 / total_bet as f64) * 100.0;
            println!(
                "{mode:?}: n={per_mode_n}  wins={wins}  RTP={rtp:.6}%  edge={:.6}%",
                100.0 - rtp
            );

            assert!(
                rtp >= 98.50,
                "{mode:?} RTP too low: {rtp:.6}%"
            );
            assert!(
                rtp <= 99.50,
                "{mode:?} RTP too high: {rtp:.6}%"
            );
        }
    }
}
