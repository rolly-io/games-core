use rolly_game_core::keno::*;
use rolly_game_core::shared::*;

// ── Binomial table ────────────────────────────────────────────

#[test]
fn binom_known_values() {
    assert_eq!(binom(0, 0), 1);
    assert_eq!(binom(5, 0), 1);
    assert_eq!(binom(10, 10), 1);
    assert_eq!(binom(10, 5), 252);
    assert_eq!(binom(20, 10), 184756);
    assert_eq!(binom(39, 10), 635_745_396);
}

#[test]
fn binom_out_of_range() {
    assert_eq!(binom(40, 10), 0);
    assert_eq!(binom(3, 5), 0);
    assert_eq!(binom(0, 1), 0);
}

#[test]
fn combo_40_10_matches_pascal() {
    let expected = binom(39, 9) + binom(39, 10);
    assert_eq!(COMBO_40_10, expected);
}

// ── Combinatorial ranking / unranking ──────────────────────────

#[test]
fn rank_min_combination() {
    let combo = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    assert_eq!(rank_combination(&combo), 0);
}

#[test]
fn rank_max_combination() {
    let combo = [30, 31, 32, 33, 34, 35, 36, 37, 38, 39];
    assert_eq!(rank_combination(&combo), COMBO_40_10 - 1);
}

#[test]
fn unrank_min() {
    assert_eq!(unrank_combination(0), [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

#[test]
fn unrank_max() {
    assert_eq!(
        unrank_combination(COMBO_40_10 - 1),
        [30, 31, 32, 33, 34, 35, 36, 37, 38, 39]
    );
}

#[test]
fn rank_unrank_roundtrip() {
    for rank in [0, 1, 100, 1000, 100_000, 1_000_000, 100_000_000, COMBO_40_10 - 1] {
        let combo = unrank_combination(rank);
        assert_eq!(
            rank_combination(&combo),
            rank,
            "roundtrip failed for rank {rank}"
        );
    }
}

#[test]
fn unrank_always_sorted_and_in_range() {
    for rank in [0, 42, 12345, 500_000_000, COMBO_40_10 - 1] {
        let combo = unrank_combination(rank);
        for i in 1..DRAW_COUNT {
            assert!(
                combo[i] > combo[i - 1],
                "not sorted/unique at rank {rank}: {combo:?}"
            );
        }
        assert!(
            *combo.last().unwrap() < NUMBERS_RANGE as u8,
            "out of range at rank {rank}: {combo:?}"
        );
    }
}

#[test]
#[should_panic(expected = "rank")]
fn unrank_out_of_range_panics() {
    unrank_combination(COMBO_40_10);
}

// ── Match counting ───────────────────────────────────────────

#[test]
fn count_matches_full() {
    let drawn = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    assert_eq!(count_matches(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9], &drawn), 10);
}

#[test]
fn count_matches_none() {
    let drawn = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    assert_eq!(count_matches(&[30, 31, 32], &drawn), 0);
}

#[test]
fn count_matches_partial() {
    let drawn = [0, 5, 10, 15, 20, 25, 30, 33, 36, 39];
    assert_eq!(count_matches(&[5, 15, 25, 38], &drawn), 3);
}

#[test]
fn count_matches_unordered_selected() {
    let drawn = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    assert_eq!(count_matches(&[9, 3, 7, 1], &drawn), 4);
}

// ── Multiplier tables ─────────────────────────────────────────

#[test]
fn all_30_tables_accessible() {
    assert_eq!(all_multiplier_rows().len(), 30);
}

#[test]
fn each_table_has_correct_length() {
    for (risk, picks, table) in all_multiplier_rows() {
        assert_eq!(
            table.len(),
            (picks + 1) as usize,
            "risk={risk} picks={picks}: expected {} entries, got {}",
            picks + 1,
            table.len()
        );
    }
}

#[test]
fn multiplier_tables_match_backend() {
    assert_eq!(get_multiplier_table(0, 1).unwrap(), &[7000, 18600]);
    assert_eq!(
        get_multiplier_table(0, 3).unwrap(),
        &[0, 10996, 13900, 260100]
    );
    assert_eq!(
        get_multiplier_table(1, 6).unwrap(),
        &[0, 0, 0, 29985, 90000, 1809300, 7100000]
    );
    assert_eq!(
        get_multiplier_table(2, 3).unwrap(),
        &[0, 0, 0, 815100]
    );
    assert_eq!(
        get_multiplier_table(2, 10).unwrap(),
        &[0, 0, 0, 0, 34986, 80000, 130000, 632000, 5000000, 8000000, 10000000]
    );
}

#[test]
fn invalid_config_returns_none() {
    assert!(get_multiplier_table(3, 1).is_none());
    assert!(get_multiplier_table(0, 0).is_none());
    assert!(get_multiplier_table(0, 11).is_none());
}

// ── Payout computation ────────────────────────────────────────

#[test]
fn payout_all_match_low_risk_pick3() {
    let random = [0u64, 0, 0, 0]; // drawn = {0,1,2,3,4,5,6,7,8,9}
    let payout = compute_payout(&random, 100 * USDT_DECIMALS, 0, &[0, 1, 2]);
    assert_eq!(payout.roll_number, 3);
    assert_eq!(payout.multiplier, 260100); // 26.01x
    assert!(payout.is_win);
    // 100_000_000 x 260100 / 10000 = 2_601_000_000
    assert_eq!(payout.win_amount, 2_601_000_000);
}

#[test]
fn payout_no_match_high_risk_pick1() {
    let random = [0u64, 0, 0, 0]; // drawn = {0..9}
    let payout = compute_payout(&random, 100 * USDT_DECIMALS, 2, &[39]);
    assert_eq!(payout.roll_number, 0);
    assert_eq!(payout.multiplier, 0);
    assert!(!payout.is_win);
    assert_eq!(payout.win_amount, 0);
}

#[test]
fn payout_sub_1x_low_risk_pick1_no_match() {
    let random = [0u64, 0, 0, 0]; // drawn = {0..9}
    let payout = compute_payout(&random, 10 * USDT_DECIMALS, 0, &[39]);
    assert_eq!(payout.roll_number, 0);
    assert_eq!(payout.multiplier, 7000); // 0.7x
    assert!(payout.is_win);
    // 10_000_000 x 7000 / 10000 = 7_000_000
    assert_eq!(payout.win_amount, 7_000_000);
}

#[test]
fn payout_max_win_cap() {
    let random = [0u64, 0, 0, 0]; // drawn = {0..9}
    let selected = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]; // 10 matches
    // low risk pick 10: 1000x (10_000_000)
    let payout = compute_payout(&random, 100 * USDT_DECIMALS, 0, &selected);
    assert_eq!(payout.multiplier, 10_000_000);
    assert_eq!(payout.win_amount, MAX_WIN);
}

#[test]
fn payout_below_max_win_not_capped() {
    let random = [0u64, 0, 0, 0];
    let payout = compute_payout(&random, USDT_DECIMALS, 0, &[0, 1, 2]);
    // 3 matches, low pick 3: 26.01x -> 1_000_000 x 260100 / 10000 = 26_010_000
    assert_eq!(payout.win_amount, 26_010_000);
    assert!(payout.win_amount < MAX_WIN);
}

#[test]
fn payout_integer_floor_division() {
    let random = [0u64, 0, 0, 0];
    // bet = 333333 atomic, multi = 18600 (1.86x)
    let payout = compute_payout(&random, 333_333, 0, &[0]); // 1 match
    // 333_333 x 18600 / 10000 = 6_199_993_800 / 10000 = 619_999 (floor)
    assert_eq!(payout.win_amount, 619_999);
}

// ── drawn_from_random ─────────────────────────────────────────

#[test]
fn drawn_from_random_deterministic() {
    let random = [12345u64, 0, 0, 0];
    assert_eq!(drawn_from_random(&random), drawn_from_random(&random));
}

#[test]
fn drawn_from_random_always_sorted_unique_in_range() {
    for val in [0, 1, 999, 123_456_789, u64::MAX] {
        let random = [val, 0, 0, 0];
        let drawn = drawn_from_random(&random);
        for i in 1..DRAW_COUNT {
            assert!(drawn[i] > drawn[i - 1]);
        }
        assert!(*drawn.last().unwrap() < NUMBERS_RANGE as u8);
    }
}

// ── Validation ────────────────────────────────────────────────

#[test]
#[should_panic(expected = "duplicates")]
fn payout_rejects_duplicate_selected() {
    let random = [0u64, 0, 0, 0];
    compute_payout(&random, USDT_DECIMALS, 0, &[5, 5]);
}

#[test]
#[should_panic(expected = "out of range")]
fn payout_rejects_out_of_range_selected() {
    let random = [0u64, 0, 0, 0];
    compute_payout(&random, USDT_DECIMALS, 0, &[40]);
}

#[test]
#[should_panic(expected = "risk must be")]
fn payout_rejects_invalid_risk() {
    let random = [0u64, 0, 0, 0];
    compute_payout(&random, USDT_DECIMALS, 3, &[0]);
}

#[test]
#[should_panic(expected = "pick_count must be")]
fn payout_rejects_empty_selected() {
    let random = [0u64, 0, 0, 0];
    compute_payout(&random, USDT_DECIMALS, 0, &[]);
}

// ── Fuzz ──────────────────────────────────────────────────────

#[test]
fn fuzz_rank_unrank_roundtrip_1000() {
    use rand::prelude::*;
    use rand::rngs::StdRng;
    let mut rng = StdRng::seed_from_u64(0xDEAD_BEEF);

    for _ in 0..1000 {
        let rank: u64 = rng.gen_range(0..COMBO_40_10);
        let combo = unrank_combination(rank);
        assert_eq!(rank_combination(&combo), rank);
        for i in 1..DRAW_COUNT {
            assert!(combo[i] > combo[i - 1]);
        }
        assert!(*combo.last().unwrap() < NUMBERS_RANGE as u8);
    }
}

#[test]
fn fuzz_payout_100_per_config() {
    use rand::prelude::*;
    use rand::rngs::StdRng;
    let mut rng = StdRng::seed_from_u64(0x0110_CAFE_0003);

    for risk in 0..3u8 {
        for picks in 1..=10u8 {
            let table = get_multiplier_table(risk, picks).unwrap();
            for _ in 0..100 {
                let random: [u64; 4] = [rng.gen(), rng.gen(), rng.gen(), rng.gen()];
                let bet: u64 = rng.gen_range(10_000..=700_000_000);

                let mut nums: Vec<u8> = (0..NUMBERS_RANGE as u8).collect();
                nums.shuffle(&mut rng);
                let selected: Vec<u8> = nums[..picks as usize].to_vec();

                let payout = compute_payout(&random, bet, risk, &selected);

                assert!(payout.roll_number <= picks as u32);
                let expected_multi = table[payout.roll_number as usize];
                assert_eq!(payout.multiplier, expected_multi);

                if expected_multi > 0 {
                    let expected_raw =
                        (bet as u128 * expected_multi as u128) / PAYOUT_DIVISOR as u128;
                    let expected_win = (expected_raw as u64).min(MAX_WIN);
                    assert_eq!(payout.win_amount, expected_win);
                    assert!(payout.is_win);
                } else {
                    assert_eq!(payout.win_amount, 0);
                    assert!(!payout.is_win);
                }
            }
        }
    }
}

// ── RTP simulation ──────────────────────────────────────────────
//
// Keno RTP depends on risk level. Low risk has higher RTP than high risk.
// Per-risk RTP should converge to the designed value for each risk tier.
//
// Default: 1B iterations. Override via env:
//   RTP_ITERATIONS=1000000 cargo test --release -p rolly-game-core rtp_simulation_keno -- --ignored --nocapture

fn rtp_iteration_count() -> u64 {
    std::env::var("RTP_ITERATIONS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1_000_000_000)
}

#[test]
#[ignore]
fn rtp_simulation_keno() {
    use rand::prelude::*;
    use rand::rngs::StdRng;
    let n = rtp_iteration_count();
    let mut rng = StdRng::seed_from_u64(42);

    struct RiskStats {
        risk: u8,
        total_bet: u128,
        total_win: u128,
        count: u64,
    }

    let mut per_risk: Vec<RiskStats> = (0..3u8)
        .map(|r| RiskStats { risk: r, total_bet: 0, total_win: 0, count: 0 })
        .collect();
    let mut total_bet: u128 = 0;
    let mut total_win: u128 = 0;

    for _ in 0..n {
        let risk: u8 = rng.gen_range(0..3);
        let picks: u8 = rng.gen_range(1..=10);
        let random: [u64; 4] = [rng.gen(), rng.gen(), rng.gen(), rng.gen()];

        let mut nums: Vec<u8> = (0..NUMBERS_RANGE as u8).collect();
        nums.shuffle(&mut rng);
        let selected: Vec<u8> = nums[..picks as usize].to_vec();

        let table = get_multiplier_table(risk, picks).unwrap();
        let max_multi = *table.iter().max().unwrap();
        let max_bet = if max_multi > 0 {
            (MAX_WIN as u128 * PAYOUT_DIVISOR as u128 / max_multi as u128) as u64
        } else {
            700_000_000
        };
        let bet: u64 = rng.gen_range(10_000..=max_bet.max(10_000));

        let payout = compute_payout(&random, bet, risk, &selected);
        let win = payout.win_amount as u128;

        total_bet += bet as u128;
        total_win += win;

        let s = &mut per_risk[risk as usize];
        s.total_bet += bet as u128;
        s.total_win += win;
        s.count += 1;
    }

    let rtp = (total_win as f64 / total_bet as f64) * 100.0;
    let risk_names = ["Low", "Medium", "High"];

    println!();
    println!("=== KENO RTP SIMULATION ===");
    println!("Iterations:  {n}");
    println!("Overall RTP: {rtp:.6}%");
    println!("House edge:  {:.6}%", 100.0 - rtp);
    println!();

    for s in &per_risk {
        let r = (s.total_win as f64 / s.total_bet as f64) * 100.0;
        println!(
            "  {:>6} risk  n={:>12}  RTP={:.6}%  edge={:.6}%",
            risk_names[s.risk as usize], s.count, r, 100.0 - r,
        );
    }
    println!();

    assert!(rtp >= 98.5, "Overall RTP too low: {rtp:.6}%");
    assert!(rtp <= 99.5, "Overall RTP too high: {rtp:.6}%");
}
