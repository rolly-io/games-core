use rolly_game_core::coinflip::*;
use rolly_game_core::shared::*;

fn random_with_roll(desired: u32) -> [u64; 4] {
    [desired as u64, 0, 0, 0]
}

// ── Roll extraction ──────────────────────────────────────────

#[test]
fn roll_even_gives_0() {
    assert_eq!(roll_from_random(&[0, 0, 0, 0]), 0);
    assert_eq!(roll_from_random(&[2, 0, 0, 0]), 0);
    assert_eq!(roll_from_random(&[100, 0, 0, 0]), 0);
}

#[test]
fn roll_odd_gives_1() {
    assert_eq!(roll_from_random(&[1, 0, 0, 0]), 1);
    assert_eq!(roll_from_random(&[3, 0, 0, 0]), 1);
    assert_eq!(roll_from_random(&[99, 0, 0, 0]), 1);
}

#[test]
fn roll_large_goldilocks_value() {
    let p_minus_1: u64 = 18_446_744_069_414_584_320;
    assert_eq!(roll_from_random(&[p_minus_1, 0, 0, 0]), 0);
    assert_eq!(roll_from_random(&[p_minus_1 - 1, 0, 0, 0]), 1);
}

// ── Win / loss ───────────────────────────────────────────────

#[test]
fn prediction_0_roll_0_wins() {
    let random = random_with_roll(0);
    let payout = compute_payout(&random, 100 * USDT_DECIMALS, 0);
    assert!(payout.is_win);
    assert_eq!(payout.roll_number, 0);
    assert_eq!(payout.multiplier, MULTIPLIER);
    // 100_000_000 * 19800 / 10000 = 198_000_000
    assert_eq!(payout.win_amount, 198_000_000);
}

#[test]
fn prediction_1_roll_1_wins() {
    let random = random_with_roll(1);
    let payout = compute_payout(&random, 100 * USDT_DECIMALS, 1);
    assert!(payout.is_win);
    assert_eq!(payout.roll_number, 1);
    assert_eq!(payout.multiplier, MULTIPLIER);
    assert_eq!(payout.win_amount, 198_000_000);
}

#[test]
fn prediction_0_roll_1_loses() {
    let random = random_with_roll(1);
    let payout = compute_payout(&random, 100 * USDT_DECIMALS, 0);
    assert!(!payout.is_win);
    assert_eq!(payout.roll_number, 1);
    assert_eq!(payout.multiplier, 0);
    assert_eq!(payout.win_amount, 0);
}

#[test]
fn prediction_1_roll_0_loses() {
    let random = random_with_roll(0);
    let payout = compute_payout(&random, 100 * USDT_DECIMALS, 1);
    assert!(!payout.is_win);
    assert_eq!(payout.roll_number, 0);
    assert_eq!(payout.multiplier, 0);
    assert_eq!(payout.win_amount, 0);
}

// ── MAX_WIN cap ──────────────────────────────────────────────

#[test]
fn max_win_cap_applied() {
    let bet = 6_000 * USDT_DECIMALS; // 6000 * 1.98 = 11880 > 10000
    let random = random_with_roll(0);
    let payout = compute_payout(&random, bet, 0);
    assert!(payout.is_win);
    assert_eq!(payout.win_amount, MAX_WIN);
}

#[test]
fn below_max_win_not_capped() {
    let bet = 50 * USDT_DECIMALS; // 50 * 1.98 = 99 USDT
    let random = random_with_roll(0);
    let payout = compute_payout(&random, bet, 0);
    assert!(payout.is_win);
    assert_eq!(payout.win_amount, 99_000_000);
    assert!(payout.win_amount < MAX_WIN);
}

// ── Zero win on loss ─────────────────────────────────────────

#[test]
fn zero_win_on_loss() {
    let random = random_with_roll(1);
    let payout = compute_payout(&random, 500 * USDT_DECIMALS, 0);
    assert!(!payout.is_win);
    assert_eq!(payout.win_amount, 0);
}

// ── u128 overflow safety ─────────────────────────────────────

#[test]
fn u128_no_overflow_max_scenario() {
    let bet = 700 * USDT_DECIMALS;
    let raw = (bet as u128 * MULTIPLIER as u128) / PAYOUT_DIVISOR as u128;
    assert!(raw < u64::MAX as u128);
    // 700_000_000 * 19800 / 10000 = 1_386_000_000
    assert_eq!(raw, 1_386_000_000u128);
}

// ── Floor division ───────────────────────────────────────────

#[test]
fn integer_floor_division() {
    let bet = 333_333u64;
    let random = random_with_roll(0);
    let payout = compute_payout(&random, bet, 0);
    // 333_333 * 19800 / 10000 = 6_599_993_400 / 10000 = 659_999 (floor)
    assert_eq!(payout.win_amount, 659_999);
}

// ── Invalid prediction panics ────────────────────────────────

#[test]
#[should_panic(expected = "prediction must be 0 or 1")]
fn prediction_2_panics() {
    compute_payout(&[0, 0, 0, 0], 1_000_000, 2);
}

#[test]
#[should_panic(expected = "prediction must be 0 or 1")]
fn prediction_255_panics() {
    compute_payout(&[0, 0, 0, 0], 1_000_000, 255);
}

// ── Zero bet ─────────────────────────────────────────────────

#[test]
fn zero_bet_zero_win() {
    let random = random_with_roll(0);
    let payout = compute_payout(&random, 0, 0);
    assert!(payout.is_win);
    assert_eq!(payout.win_amount, 0);
}

// ── RTP simulation ───────────────────────────────────────────

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

    fn uncapped_win(bet: u64, won: bool) -> u128 {
        if won {
            (bet as u128 * MULTIPLIER as u128) / PAYOUT_DIVISOR as u128
        } else {
            0
        }
    }

    #[test]
    #[ignore]
    fn rtp_simulation_coinflip() {
        let n = iteration_count();
        let mut rng = StdRng::seed_from_u64(42);
        let mut total_bet: u128 = 0;
        let mut total_win: u128 = 0;
        let mut wins: u64 = 0;

        for _ in 0..n {
            let prediction: u8 = rng.gen_range(0..=1);
            let bet: u64 = rng.gen_range(10_000..=700_000_000);
            let random: [u64; 4] = [rng.gen(), rng.gen(), rng.gen(), rng.gen()];

            let roll = roll_from_random(&random);
            let won = roll == prediction as u32;
            let win = uncapped_win(bet, won);

            total_bet += bet as u128;
            total_win += win;
            if won { wins += 1; }
        }

        let rtp = (total_win as f64 / total_bet as f64) * 100.0;

        println!();
        println!("=== COINFLIP RTP SIMULATION ===");
        println!("Iterations:  {n}");
        println!("Wins:        {wins}");
        println!("RTP:         {rtp:.6}%");
        println!("Expected:    99.0000%");
        println!();

        assert!(rtp >= 98.98, "RTP too low: {rtp:.6}%");
        assert!(rtp <= 99.02, "RTP too high: {rtp:.6}%");
    }
}
