use rolly_game_core::plinko::*;
use rolly_game_core::shared::*;

fn random_with_bits(low_bits: u64) -> [u64; 4] {
    [low_bits, 0, 0, 0]
}

// ── Table structure ────────────────────────────────────────────

#[test]
fn all_30_tables_accessible() {
    let rows = all_multiplier_rows();
    assert_eq!(rows.len(), 30, "27 normal + 3 extreme");
}

#[test]
fn each_table_has_correct_length() {
    for (sector, rows, is_extreme, table) in all_multiplier_rows() {
        assert_eq!(
            table.len(),
            (rows + 1) as usize,
            "sector={sector} rows={rows} extreme={is_extreme}: expected {} entries, got {}",
            rows + 1,
            table.len()
        );
    }
}

#[test]
fn all_multipliers_positive() {
    for (sector, rows, is_extreme, table) in all_multiplier_rows() {
        for (i, &m) in table.iter().enumerate() {
            assert!(m > 0,
                "zero multiplier at sector={sector} rows={rows} extreme={is_extreme} bucket={i}");
        }
    }
}

// ── Valid config checks ────────────────────────────────────────

#[test]
fn valid_normal_configs() {
    for sector in 0..3 {
        for rows in 8..=16 {
            assert!(is_valid_config(sector, rows, false),
                "sector={sector} rows={rows} should be valid normal");
        }
    }
}

#[test]
fn valid_extreme_configs() {
    assert!(is_valid_config(0, 12, true));
    assert!(is_valid_config(1, 14, true));
    assert!(is_valid_config(2, 16, true));
}

#[test]
fn invalid_extreme_configs() {
    assert!(!is_valid_config(0, 8, true));
    assert!(!is_valid_config(0, 16, true));
    assert!(!is_valid_config(1, 12, true));
    assert!(!is_valid_config(2, 14, true));
}

#[test]
fn invalid_sector() {
    assert!(!is_valid_config(3, 8, false));
    assert!(!is_valid_config(255, 12, true));
}

#[test]
fn invalid_rows() {
    assert!(!is_valid_config(0, 7, false));
    assert!(!is_valid_config(0, 17, false));
}

// ── Path extraction ────────────────────────────────────────────

#[test]
fn path_all_left() {
    let random = random_with_bits(0);
    let path = path_from_random(&random, 8);
    assert_eq!(path, vec![0; 8]);
    assert_eq!(bucket_index(&path), 0);
}

#[test]
fn path_all_right_8() {
    let random = random_with_bits(0xFF);
    let path = path_from_random(&random, 8);
    assert_eq!(path, vec![1; 8]);
    assert_eq!(bucket_index(&path), 8);
}

#[test]
fn path_all_right_16() {
    let random = random_with_bits(0xFFFF);
    let path = path_from_random(&random, 16);
    assert_eq!(path, vec![1; 16]);
    assert_eq!(bucket_index(&path), 16);
}

#[test]
fn path_alternating() {
    let random = random_with_bits(0b10101010);
    let path = path_from_random(&random, 8);
    assert_eq!(path, vec![0, 1, 0, 1, 0, 1, 0, 1]);
    assert_eq!(bucket_index(&path), 4);
}

#[test]
fn bucket_index_matches_popcount() {
    let random = random_with_bits(0b110100101011u64);
    assert_eq!(bucket_index_from_random(&random, 12), 7);

    let path = path_from_random(&random, 12);
    assert_eq!(bucket_index(&path), 7);
}

#[test]
fn path_only_uses_rows_bits() {
    let random = random_with_bits(0xFFFF_FFFF_FFFF_FFFFu64);
    assert_eq!(bucket_index_from_random(&random, 8), 8);
    assert_eq!(bucket_index_from_random(&random, 12), 12);
    assert_eq!(bucket_index_from_random(&random, 16), 16);
}

// ── Multiplier lookups ─────────────────────────────────────────

#[test]
fn lookup_sector0_rows8_bucket0() {
    assert_eq!(lookup_multiplier(0, 8, false, 0), 56000);
}

#[test]
fn lookup_sector0_rows8_bucket_last() {
    assert_eq!(lookup_multiplier(0, 8, false, 8), 56000);
}

#[test]
fn lookup_sector0_rows8_bucket_center() {
    assert_eq!(lookup_multiplier(0, 8, false, 4), 5000);
}

#[test]
fn lookup_sector2_rows12_symmetric() {
    assert_eq!(lookup_multiplier(2, 12, false, 0), 1681000);
    assert_eq!(lookup_multiplier(2, 12, false, 12), 1681000);
}

#[test]
fn lookup_sector0_rows15_symmetric() {
    assert_eq!(lookup_multiplier(0, 15, false, 0), 177000);
    assert_eq!(lookup_multiplier(0, 15, false, 15), 177000);
}

#[test]
fn lookup_extreme_max_multiplier() {
    assert_eq!(lookup_multiplier(0, 12, true, 0), 10_000_000);
    assert_eq!(lookup_multiplier(1, 14, true, 0), 35_000_000);
    assert_eq!(lookup_multiplier(2, 16, true, 0), 100_000_000);
}

#[test]
fn lookup_extreme_min_multiplier() {
    assert_eq!(lookup_multiplier(0, 12, true, 5), 1000);
    assert_eq!(lookup_multiplier(1, 14, true, 7), 1000);
    assert_eq!(lookup_multiplier(2, 16, true, 7), 1000);
}

// ── Payout computation ─────────────────────────────────────────

#[test]
fn payout_bucket0_sector0_rows8() {
    let random = random_with_bits(0);
    let payout = compute_payout(&random, 100 * USDT_DECIMALS, 0, 8, false);
    assert_eq!(payout.roll_number, 0);
    assert_eq!(payout.multiplier, 56000);
    assert!(payout.is_win);
    // 100_000_000 * 56000 / 10000 = 560_000_000
    assert_eq!(payout.win_amount, 560_000_000);
}

#[test]
fn payout_middle_bucket_sub_1x() {
    // 4 bits set in low 8 bits => bucket 4 (0.5x for sector 0 rows 8)
    let random = random_with_bits(0b00001111);
    let payout = compute_payout(&random, 100 * USDT_DECIMALS, 0, 8, false);
    assert_eq!(payout.roll_number, 4);
    assert_eq!(payout.multiplier, 5000);
    assert!(!payout.is_win);
    // 100_000_000 * 5000 / 10000 = 50_000_000 (0.5x)
    assert_eq!(payout.win_amount, 50_000_000);
}

#[test]
fn payout_always_nonzero() {
    for (sector, rows, is_extreme, table) in all_multiplier_rows() {
        let min_multi = *table.iter().min().unwrap();
        assert!(min_multi > 0);

        let bet = 10_000u64; // 0.01 USDT (minimum)
        let raw = (bet as u128 * min_multi as u128) / PAYOUT_DIVISOR as u128;
        assert!(raw > 0,
            "zero payout for min bet at sector={sector} rows={rows} extreme={is_extreme}: multi={min_multi}");
    }
}

// ── MAX_WIN cap ────────────────────────────────────────────────

#[test]
fn max_win_cap_extreme() {
    let random = random_with_bits(0);
    // extreme sector 2 rows 16, bucket 0: 10000x multiplier
    let payout = compute_payout(&random, 100 * USDT_DECIMALS, 2, 16, true);
    assert_eq!(payout.multiplier, 100_000_000);
    // 100 USDT * 10000x = 1_000_000 USDT >> MAX_WIN (10000 USDT)
    assert_eq!(payout.win_amount, MAX_WIN);
}

#[test]
fn below_max_win_not_capped() {
    let random = random_with_bits(0);
    // sector 0 rows 8 bucket 0: 5.6x
    let payout = compute_payout(&random, 10 * USDT_DECIMALS, 0, 8, false);
    // 10_000_000 * 56000 / 10000 = 56_000_000
    assert_eq!(payout.win_amount, 56_000_000);
    assert!(payout.win_amount < MAX_WIN);
}

// ── Floor division ─────────────────────────────────────────────

#[test]
fn integer_floor_division() {
    let random = random_with_bits(0);
    // bet = 333333 atomic (0.333333 USDT), multi = 56000 (5.6x)
    let payout = compute_payout(&random, 333_333, 0, 8, false);
    // 333_333 * 56000 / 10000 = 18_666_648_000 / 10000 = 1_866_664 (floor)
    assert_eq!(payout.win_amount, 1_866_664);
}

// ── u128 no overflow ───────────────────────────────────────────

#[test]
fn u128_no_overflow_max_scenario() {
    // max bet = 700 USDT, max multi = 100_000_000 (extreme 10000x)
    let bet = 700 * USDT_DECIMALS;
    let multi: u64 = 100_000_000;
    let raw = bet as u128 * multi as u128 / PAYOUT_DIVISOR as u128;
    // 700_000_000 * 100_000_000 / 10_000 = 7_000_000_000_000
    assert!(raw < u64::MAX as u128);
    assert_eq!(raw, 7_000_000_000_000u128);
}

// ── is_win semantics ───────────────────────────────────────────

#[test]
fn is_win_true_when_multi_gte_1x() {
    let random = random_with_bits(0);
    let payout = compute_payout(&random, USDT_DECIMALS, 0, 8, false);
    assert!(payout.multiplier >= PAYOUT_DIVISOR);
    assert!(payout.is_win);
}

#[test]
fn is_win_false_when_multi_lt_1x() {
    // 4 bits set => bucket 4, sector 0 rows 8 => 0.5x
    let random = random_with_bits(0b00001111);
    let payout = compute_payout(&random, USDT_DECIMALS, 0, 8, false);
    assert_eq!(payout.multiplier, 5000); // 0.5x
    assert!(!payout.is_win);
    assert!(payout.win_amount > 0, "plinko still pays out even on sub-1x");
}

// ── Specific golden values ─────────────────────────────────────

#[test]
fn golden_sector1_rows16_bucket8() {
    // middle bucket, sector 1 rows 16: 0.3x
    let random = random_with_bits(0b0000000011111111); // 8 right moves in first 16 bits
    let payout = compute_payout(&random, 50 * USDT_DECIMALS, 1, 16, false);
    assert_eq!(payout.roll_number, 8);
    assert_eq!(payout.multiplier, 3000);
    assert!(!payout.is_win);
    // 50_000_000 * 3000 / 10000 = 15_000_000
    assert_eq!(payout.win_amount, 15_000_000);
}

#[test]
fn golden_sector2_rows16_extreme_bucket0() {
    let random = random_with_bits(0);
    let payout = compute_payout(&random, 1 * USDT_DECIMALS, 2, 16, true);
    assert_eq!(payout.roll_number, 0);
    assert_eq!(payout.multiplier, 100_000_000);
    assert!(payout.is_win);
    // 1_000_000 * 100_000_000 / 10000 = 10_000_000_000 = MAX_WIN exactly
    assert_eq!(payout.win_amount, MAX_WIN);
}

// ── Random fuzzing ─────────────────────────────────────────────

#[test]
fn fuzz_all_configs_100_random() {
    use rand::prelude::*;
    use rand::rngs::StdRng;
    let mut rng = StdRng::seed_from_u64(0x0110_CAFE_0000);

    for (sector, rows, is_extreme, table) in all_multiplier_rows() {
        for _ in 0..100 {
            let random: [u64; 4] = [rng.gen(), rng.gen(), rng.gen(), rng.gen()];
            let bet: u64 = rng.gen_range(10_000..=700_000_000);

            let payout = compute_payout(&random, bet, sector, rows, is_extreme);

            assert!(payout.roll_number <= rows);
            let expected_multi = table[payout.roll_number as usize];
            assert_eq!(payout.multiplier, expected_multi);

            let expected_raw = (bet as u128 * expected_multi as u128) / PAYOUT_DIVISOR as u128;
            let expected_win = (expected_raw as u64).min(MAX_WIN);
            assert_eq!(payout.win_amount, expected_win);
            assert_eq!(payout.is_win, expected_multi >= PAYOUT_DIVISOR);
        }
    }
}

// ── RTP simulation ──────────────────────────────────────────────
//
// Plinko always pays out (even sub-1x), so RTP is computed as
// sum(win_amount) / sum(bet_amount) across all configs uniformly.
//
// Default: 1B iterations. Override via env:
//   RTP_ITERATIONS=1000000 cargo test --release -p rolly-game-core rtp_simulation_plinko -- --ignored --nocapture

fn rtp_iteration_count() -> u64 {
    std::env::var("RTP_ITERATIONS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1_000_000_000)
}

#[test]
#[ignore]
fn rtp_simulation_plinko() {
    use rand::prelude::*;
    use rand::rngs::StdRng;
    let n = rtp_iteration_count();
    let mut rng = StdRng::seed_from_u64(42);

    let configs = all_multiplier_rows();

    struct Stats {
        label: String,
        total_bet: u128,
        total_win: u128,
        count: u64,
    }

    let mut per_config: Vec<Stats> = configs
        .iter()
        .map(|(s, r, e, _)| Stats {
            label: format!("s{}r{}{}", s, r, if *e { "X" } else { "" }),
            total_bet: 0,
            total_win: 0,
            count: 0,
        })
        .collect();

    let mut total_bet: u128 = 0;
    let mut total_win: u128 = 0;

    for _ in 0..n {
        let cfg_idx = rng.gen_range(0..configs.len());
        let (sector, rows, is_extreme, table) = configs[cfg_idx];
        let max_multi = *table.iter().max().unwrap();
        let max_bet = if max_multi > 0 {
            (MAX_WIN as u128 * PAYOUT_DIVISOR as u128 / max_multi as u128) as u64
        } else {
            700_000_000
        };
        let bet: u64 = rng.gen_range(10_000..=max_bet.max(10_000));
        let random: [u64; 4] = [rng.gen(), rng.gen(), rng.gen(), rng.gen()];

        let payout = compute_payout(&random, bet, sector, rows, is_extreme);
        let win = payout.win_amount as u128;

        total_bet += bet as u128;
        total_win += win;

        let s = &mut per_config[cfg_idx];
        s.total_bet += bet as u128;
        s.total_win += win;
        s.count += 1;
    }

    let rtp = (total_win as f64 / total_bet as f64) * 100.0;

    println!();
    println!("=== PLINKO RTP SIMULATION ===");
    println!("Iterations:  {n}");
    println!("RTP:         {rtp:.6}%");
    println!("House edge:  {:.6}%", 100.0 - rtp);
    println!();

    for s in &per_config {
        if s.count == 0 { continue; }
        let cfg_rtp = (s.total_win as f64 / s.total_bet as f64) * 100.0;
        println!(
            "  {:>8}  n={:>10}  RTP={:.4}%  edge={:.4}%",
            s.label, s.count, cfg_rtp, 100.0 - cfg_rtp,
        );
    }
    println!();

    // just report, no assertion on overall since RTP differs per config by design
}

#[test]
#[ignore]
fn rtp_simulation_plinko_extreme_100m() {
    use rand::prelude::*;
    use rand::rngs::StdRng;
    let n = 500_000_000u64;

    let extreme_configs: [(u8, u32, &str); 2] = [
        (1, 14, "s1r14X"),
        (2, 16, "s2r16X"),
    ];

    println!();
    println!("=== PLINKO EXTREME RTP (500M bets each, no cap) ===");
    println!();

    for &(sector, rows, label) in &extreme_configs {
        let mut rng = StdRng::seed_from_u64(sector as u64 * 1000 + rows as u64);
        let mut total_bet: u128 = 0;
        let mut total_win: u128 = 0;
        let mut wins: u64 = 0;

        let table = get_multiplier_row(sector, rows, true).unwrap();
        let max_multi = *table.iter().max().unwrap();
        let max_bet = (MAX_WIN as u128 * PAYOUT_DIVISOR as u128 / max_multi as u128) as u64;

        for _ in 0..n {
            let bet: u64 = rng.gen_range(10_000..=max_bet.max(10_000));
            let random: [u64; 4] = [rng.gen(), rng.gen(), rng.gen(), rng.gen()];
            let payout = compute_payout(&random, bet, sector, rows, true);

            total_bet += bet as u128;
            total_win += payout.win_amount as u128;
            if payout.is_win { wins += 1; }
        }

        let rtp = (total_win as f64 / total_bet as f64) * 100.0;
        let win_rate = (wins as f64 / n as f64) * 100.0;
        println!(
            "  {label}  n={n}  wins={wins}  win_rate={win_rate:.6}%  RTP={rtp:.6}%  edge={:.6}%",
            100.0 - rtp,
        );
    }
    println!();
}
