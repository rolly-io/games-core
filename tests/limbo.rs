use rolly_game_core::limbo::*;
use rolly_game_core::shared::*;
use rand::prelude::*;
use rand::rngs::StdRng;

fn random_with_v(desired_v: u64) -> [u64; 4] {
    [desired_v & 0xFFFF_FFFF, 0, 0, 0]
}

// ── Multiplier extraction ──────────────────────────────────────

#[test]
fn multiplier_v_zero_clamps_to_display_floor() {
    // True raw at v=0 is 98 (0.98x); display floor lifts it to 1.00x, which is
    // strictly below the minimum target (1.01x) so a loss never reads as a win.
    let r = random_with_v(0);
    assert_eq!(multiplier_from_random(&r), MIN_DISPLAY_MULTI_X100);
}

#[test]
fn multiplier_v_max_clamps_to_max() {
    let r = random_with_v(POW2_32 - 1);
    assert_eq!(multiplier_from_random(&r), MAX_MULTI_X100);
}

#[test]
fn multiplier_exact_200_rtp98() {
    // denom = floor(98 * 2^32 / 200) = 2104533975 → v = 2^32 - 2104533975 = 2190433321
    let r = random_with_v(2_190_433_321);
    assert_eq!(multiplier_from_random(&r), 200);
}

#[test]
fn multiplier_ignores_upper_bits() {
    let r = [0x1_0000_0000u64 | (POW2_32 - 1), 0, 0, 0];
    assert_eq!(multiplier_from_random(&r), MAX_MULTI_X100);
}

// ── Win / loss ─────────────────────────────────────────────────

#[test]
fn win_low_prediction_high_multi() {
    let r = random_with_v(POW2_32 - 1);
    let p = compute_payout(&r, 100 * USDT_DECIMALS, 200);
    assert!(p.is_win);
    assert_eq!(p.roll_number, MAX_MULTI_X100 as u32);
    assert_eq!(p.win_amount, 200_000_000); // 100 * 2.00x
    assert_eq!(p.multiplier, 20_000);
}

#[test]
fn lose_high_prediction_low_multi() {
    let r = random_with_v(0);
    let p = compute_payout(&r, 100 * USDT_DECIMALS, 200);
    assert!(!p.is_win);
    assert_eq!(p.win_amount, 0);
    assert_eq!(p.multiplier, 0);
    assert_eq!(p.roll_number, MIN_DISPLAY_MULTI_X100 as u32);
}

#[test]
fn win_at_exact_threshold() {
    // v=2190433321 → denom=2104533975 → multi=200 with rtp=98
    // prediction=200 → lhs = 200 * 2104533975 = 420906795000 <= rhs = 420906795008 → WIN
    let r = random_with_v(2_190_433_321);
    let p = compute_payout(&r, 50 * USDT_DECIMALS, 200);
    assert!(p.is_win);
    assert_eq!(p.roll_number, 200);
    assert_eq!(p.win_amount, 100_000_000); // 50 * 2.00x
}

#[test]
fn lose_just_above_threshold() {
    let r = random_with_v(2_190_433_321);
    let p = compute_payout(&r, 50 * USDT_DECIMALS, 201);
    assert!(!p.is_win);
    assert_eq!(p.win_amount, 0);
    assert_eq!(p.roll_number, 200);
}

// ── MAX_WIN cap ────────────────────────────────────────────────

#[test]
fn max_win_cap_applied() {
    let r = random_with_v(POW2_32 - 1);
    let p = compute_payout(&r, 100 * USDT_DECIMALS, 999_999);
    assert!(p.is_win);
    assert_eq!(p.win_amount, MAX_WIN);
}

#[test]
fn below_max_win_not_capped() {
    let r = random_with_v(POW2_32 - 1);
    let p = compute_payout(&r, 10 * USDT_DECIMALS, 200);
    assert!(p.is_win);
    assert_eq!(p.win_amount, 20_000_000); // 10 * 2.00x
    assert!(p.win_amount < MAX_WIN);
}

// ── Zero bet ───────────────────────────────────────────────────

#[test]
fn zero_bet_zero_win() {
    let r = random_with_v(POW2_32 - 1);
    let p = compute_payout(&r, 0, 200);
    assert!(p.is_win);
    assert_eq!(p.win_amount, 0);
}

// ── Validation panics ──────────────────────────────────────────

#[test]
#[should_panic(expected = "prediction_x100 must be in")]
fn prediction_below_min_panics() {
    compute_payout(&[0, 0, 0, 0], 1_000_000, 100);
}

#[test]
#[should_panic(expected = "prediction_x100 must be in")]
fn prediction_above_max_panics() {
    compute_payout(&[0, 0, 0, 0], 1_000_000, 1_000_000);
}

// ── u128 overflow safety ───────────────────────────────────────

#[test]
fn u128_no_overflow_max_scenario() {
    let bet = 700 * USDT_DECIMALS;
    let prediction_x10000 = MAX_MULTI_X100 * 100;
    let raw = (bet as u128 * prediction_x10000 as u128) / PAYOUT_DIVISOR as u128;
    assert!(raw < u64::MAX as u128);
}

// ── Floor division ─────────────────────────────────────────────

#[test]
fn integer_floor_division() {
    let r = random_with_v(POW2_32 - 1);
    let p = compute_payout(&r, 333_333, 300);
    assert!(p.is_win);
    assert_eq!(p.win_amount, 999_999);
}

#[test]
fn floor_loses_remainder() {
    let r = random_with_v(POW2_32 - 1);
    let p = compute_payout(&r, 1, 300);
    assert!(p.is_win);
    assert_eq!(p.win_amount, 3);

    let p2 = compute_payout(&r, 1, 101);
    assert!(p2.is_win);
    assert_eq!(p2.win_amount, 1);
}

// ── Win check equivalence with multiplier ──────────────────────

#[test]
fn win_check_matches_multiplier_comparison_1k() {
    let mut rng = StdRng::seed_from_u64(99);
    for _ in 0..1000 {
        let pred = rng.gen_range(MIN_MULTI_X100 as u32..=MAX_MULTI_X100 as u32);
        let random: [u64; 4] = [rng.gen(), rng.gen(), rng.gen(), rng.gen()];

        let multi = multiplier_from_random(&random);
        let v = random[0] & 0xFFFF_FFFF;
        let denom = POW2_32 - v;
        let lhs = pred as u128 * denom as u128;
        let rhs = RTP_PERCENT as u128 * POW2_32 as u128;
        let won_comparison = lhs <= rhs;

        let won_multi = pred as u64 <= multi;
        assert_eq!(
            won_comparison, won_multi,
            "mismatch: pred={pred} multi={multi} denom={denom} v={v}"
        );
    }
}

// ── RTP simulation ─────────────────────────────────────────────

mod rtp_simulation {
    use super::*;

    const DEFAULT_ITERATIONS: u64 = 1_000_000_000;

    fn iteration_count() -> u64 {
        std::env::var("RTP_ITERATIONS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_ITERATIONS)
    }

    fn uncapped_win(bet: u64, prediction_x100: u32, won: bool) -> u128 {
        if won {
            let prediction_x10000 = prediction_x100 as u128 * 100;
            (bet as u128 * prediction_x10000) / PAYOUT_DIVISOR as u128
        } else {
            0
        }
    }

    #[test]
    #[ignore]
    fn rtp_simulation_limbo() {
        let n = iteration_count();
        let mut rng = StdRng::seed_from_u64(42);

        let mut total_bet: u128 = 0;
        let mut total_win: u128 = 0;
        let mut count: u64 = 0;
        let mut wins: u64 = 0;

        for _ in 0..n {
            let pred = rng.gen_range(MIN_MULTI_X100 as u32..=MAX_MULTI_X100 as u32);
            let bet: u64 = rng.gen_range(10_000..=700_000_000);
            let random: [u64; 4] = [rng.gen(), rng.gen(), rng.gen(), rng.gen()];

            let v = random[0] & 0xFFFF_FFFF;
            let denom = POW2_32 - v;
            let won = (pred as u128 * denom as u128) <= (RTP_PERCENT as u128 * POW2_32 as u128);
            let win = uncapped_win(bet, pred, won);

            total_bet += bet as u128;
            total_win += win;
            count += 1;
            if won { wins += 1; }
        }

        let rtp = if total_bet > 0 {
            (total_win as f64 / total_bet as f64) * 100.0
        } else {
            0.0
        };

        println!("\n=== LIMBO RTP SIMULATION ===");
        println!("Iterations: {n}");
        println!(
            "  RTP={RTP_PERCENT}  n={count:>12}  wins={wins:>12}  actual={rtp:.6}%  edge={:.6}%",
            100.0 - rtp,
        );

        assert!(
            rtp >= 98.0,
            "RTP too low: {rtp:.6}%",
        );
        assert!(
            rtp <= 100.0,
            "RTP too high: {rtp:.6}%",
        );
        println!();
    }
}
