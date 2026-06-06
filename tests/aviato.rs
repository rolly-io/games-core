use rolly_game_core::aviato::*;
use rolly_game_core::shared::*;
use rand::prelude::*;
use rand::rngs::StdRng;

fn random_with_v(desired_v: u64) -> [u64; 4] {
    [desired_v & 0xFFFF_FFFF, 0, 0, 0]
}

// ── Aviato point extraction ───────────────────────────────────

#[test]
fn aviato_v_zero_clamps_to_min() {
    let r = random_with_v(0);
    assert_eq!(aviato_point_from_random(&r), MIN_AVIATO_X100);
}

#[test]
fn aviato_v_max_clamps_to_max() {
    let r = random_with_v(POW2_32 - 1);
    assert_eq!(aviato_point_from_random(&r), MAX_MULTI_X100);
}

#[test]
fn aviato_ignores_upper_bits() {
    let r = [0x1_0000_0000u64 | (POW2_32 - 1), 0, 0, 0];
    assert_eq!(aviato_point_from_random(&r), MAX_MULTI_X100);
}

#[test]
fn aviato_point_200() {
    let v_lo = POW2_32 - (RTP_PERCENT as u64 * POW2_32 / 200);
    let r = random_with_v(v_lo);
    assert_eq!(aviato_point_from_random(&r), 200);
}

// ── Win / loss ───────────────────────────────────────────────

#[test]
fn win_cashout_below_aviato() {
    let r = random_with_v(POW2_32 - 1); // aviato = 1_000_000
    let p = compute_payout(&r, 100 * USDT_DECIMALS, 200);
    assert!(p.is_win);
    assert_eq!(p.win_amount, 200_000_000); // 100 * 2.00x
    assert_eq!(p.multiplier, 20_000);      // 200 * 100
}

#[test]
fn loss_no_cashout() {
    let r = random_with_v(POW2_32 - 1);
    let p = compute_payout(&r, 100 * USDT_DECIMALS, 0);
    assert!(!p.is_win);
    assert_eq!(p.win_amount, 0);
    assert_eq!(p.multiplier, 0);
}

#[test]
fn loss_cashout_above_aviato() {
    let r = random_with_v(0); // aviato_x100 = 100 (1.00x)
    let p = compute_payout(&r, 100 * USDT_DECIMALS, 200);
    assert!(!p.is_win);
    assert_eq!(p.win_amount, 0);
    assert_eq!(p.roll_number, 100);
}

#[test]
fn instant_aviato_min_cashout_loses() {
    // v=0 → aviato = 100 (1.00x). Min cashout = 101 (1.01x) > 100 → loss.
    let r = random_with_v(0);
    let p = compute_payout(&r, 100 * USDT_DECIMALS, MIN_CASHOUT_X100 as u32);
    assert!(!p.is_win);
    assert_eq!(p.win_amount, 0);
}

#[test]
fn min_cashout_wins_when_aviato_high() {
    // aviato >> 101 → cashout 1.01x wins. Payout = 100 * 101/100 = 101 USDT.
    let r = random_with_v(POW2_32 - 1);
    let p = compute_payout(&r, 100 * USDT_DECIMALS, MIN_CASHOUT_X100 as u32);
    assert!(p.is_win);
    assert_eq!(p.win_amount, 101_000_000);
}

// ── MAX_WIN cap ──────────────────────────────────────────────

#[test]
fn max_win_cap_applied() {
    let r = random_with_v(POW2_32 - 1); // aviato = max
    let bet = 1000 * USDT_DECIMALS;
    let p = compute_payout(&r, bet, MAX_MULTI_X100 as u32);
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

// ── Validation panics ────────────────────────────────────────

#[test]
#[should_panic(expected = "cashout_x100 must be 0")]
fn cashout_below_min_panics() {
    compute_payout(&[0, 0, 0, 0], 1_000_000, 50);
}

#[test]
#[should_panic(expected = "cashout_x100 must be 0")]
fn cashout_100_panics() {
    // 100 (1.00x) is below MIN_CASHOUT (101)
    compute_payout(&[0, 0, 0, 0], 1_000_000, 100);
}

#[test]
#[should_panic(expected = "cashout_x100 must be 0")]
fn cashout_above_max_panics() {
    compute_payout(&[0, 0, 0, 0], 1_000_000, MAX_MULTI_X100 as u32 + 1);
}

#[test]
fn cashout_zero_does_not_panic() {
    let p = compute_payout(&[0, 0, 0, 0], 1_000_000, 0);
    assert!(!p.is_win);
}

#[test]
fn cashout_min_valid_does_not_panic() {
    let r = random_with_v(POW2_32 - 1); // aviato >> 101
    let p = compute_payout(&r, 1_000_000, MIN_CASHOUT_X100 as u32);
    assert!(p.is_win);
}

#[test]
fn cashout_max_valid_does_not_panic() {
    let r = random_with_v(POW2_32 - 1);
    let p = compute_payout(&r, 1_000_000, MAX_MULTI_X100 as u32);
    assert!(p.is_win);
}

// ── Zero bet ─────────────────────────────────────────────────

#[test]
fn zero_bet_zero_win() {
    let r = random_with_v(POW2_32 - 1);
    let p = compute_payout(&r, 0, 200);
    assert!(p.is_win);
    assert_eq!(p.win_amount, 0);
}

// ── u128 overflow safety ─────────────────────────────────────

#[test]
fn u128_no_overflow_max_scenario() {
    let bet = 700 * USDT_DECIMALS;
    let raw = (bet as u128 * MAX_MULTI_X100 as u128) / AVIATO_PAYOUT_DIVISOR as u128;
    assert!(raw < u64::MAX as u128);
}

// ── Floor division ───────────────────────────────────────────

#[test]
fn integer_floor_division() {
    let r = random_with_v(POW2_32 - 1);
    let p = compute_payout(&r, 333_333, 300);
    assert!(p.is_win);
    // 333_333 * 300 / 100 = 999_999
    assert_eq!(p.win_amount, 999_999);
}

#[test]
fn floor_loses_remainder() {
    let r = random_with_v(POW2_32 - 1);
    // 1 * 300 / 100 = 3
    let p = compute_payout(&r, 1, 300);
    assert!(p.is_win);
    assert_eq!(p.win_amount, 3);

    // 1 * 101 / 100 = 1 (floor of 1.01)
    let p2 = compute_payout(&r, 1, MIN_CASHOUT_X100 as u32);
    assert!(p2.is_win);
    assert_eq!(p2.win_amount, 1);
}

// ── Distribution sanity ──────────────────────────────────────

#[test]
fn aviato_distribution_median_approx() {
    let mut rng = StdRng::seed_from_u64(42);
    let n = 100_000;
    let above_200 = (0..n)
        .filter(|_| {
            let random: [u64; 4] = [rng.gen(), rng.gen(), rng.gen(), rng.gen()];
            aviato_point_from_random(&random) >= 200
        })
        .count();
    let ratio = above_200 as f64 / n as f64;
    assert!(
        (ratio - 0.495).abs() < 0.01,
        "P(aviato >= 2.00x) should be ~49.5%, got {:.2}%",
        ratio * 100.0
    );
}

#[test]
fn aviato_distribution_10x_check() {
    let mut rng = StdRng::seed_from_u64(123);
    let n = 200_000;
    let above_1000 = (0..n)
        .filter(|_| {
            let random: [u64; 4] = [rng.gen(), rng.gen(), rng.gen(), rng.gen()];
            aviato_point_from_random(&random) >= 1000
        })
        .count();
    let ratio = above_1000 as f64 / n as f64;
    assert!(
        (ratio - 0.099).abs() < 0.005,
        "P(aviato >= 10.00x) should be ~9.9%, got {:.2}%",
        ratio * 100.0
    );
}

#[test]
fn aviato_distribution_instant_rate() {
    // P(aviato_x100 == MIN_AVIATO_X100) ≈ 1% (aviato at 1.00x → all lose)
    let mut rng = StdRng::seed_from_u64(77);
    let n = 200_000;
    let instant = (0..n)
        .filter(|_| {
            let random: [u64; 4] = [rng.gen(), rng.gen(), rng.gen(), rng.gen()];
            aviato_point_from_random(&random) == MIN_AVIATO_X100
        })
        .count();
    let ratio = instant as f64 / n as f64;
    assert!(
        (ratio - 0.0198).abs() < 0.005,
        "P(aviato == 1.00x) should be ~1.98%, got {:.2}%",
        ratio * 100.0
    );
}

// ── Multiplier field uses x10000 scale ───────────────────────

#[test]
fn multiplier_field_x10000_scale() {
    let r = random_with_v(POW2_32 - 1);
    let p = compute_payout(&r, USDT_DECIMALS, 250); // 2.50x
    assert!(p.is_win);
    assert_eq!(p.multiplier, 25_000); // 250 * 100
    assert_eq!(p.win_amount, 2_500_000); // 1 USDT * 2.5
}

// ── RTP simulation ───────────────────────────────────────────

mod rtp_simulation {
    use super::*;

    const DEFAULT_ITERATIONS: u64 = 500_000_000;

    fn iteration_count() -> u64 {
        std::env::var("RTP_ITERATIONS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_ITERATIONS)
    }

    #[test]
    #[ignore]
    fn rtp_simulation_aviato() {
        let n = iteration_count();
        let mut rng = StdRng::seed_from_u64(42);

        let mut total_bet: u128 = 0;
        let mut total_win: u128 = 0;
        let mut count: u64 = 0;
        let mut wins: u64 = 0;

        for _ in 0..n {
            let cashout = rng.gen_range(MIN_CASHOUT_X100 as u32..=10_000u32);
            let bet: u64 = rng.gen_range(10_000..=700_000_000);
            let random: [u64; 4] = [rng.gen(), rng.gen(), rng.gen(), rng.gen()];

            let aviato_x100 = aviato_point_from_random(&random);
            let won = (cashout as u64) <= aviato_x100;
            let win = if won {
                (bet as u128 * cashout as u128) / AVIATO_PAYOUT_DIVISOR as u128
            } else {
                0
            };

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

        println!("\n=== AVIATO RTP SIMULATION ===");
        println!("Iterations: {n}");
        println!(
            "  RTP=99  n={count:>12}  wins={wins:>12}  actual={rtp:.6}%  edge={:.6}%",
            100.0 - rtp,
        );

        assert!(rtp >= 98.0, "RTP too low: {rtp:.6}%");
        assert!(rtp <= 100.0, "RTP too high: {rtp:.6}%");
        println!();
    }
}
