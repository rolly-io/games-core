use crate::shared::{GamePayout, MAX_WIN, PAYOUT_DIVISOR};

pub const PLINKO_GAME_ID: u8 = 4;
pub const MIN_ROWS: u32 = 8;
pub const MAX_ROWS: u32 = 16;
pub const NUM_SECTORS: u8 = 3;

// ── Normal multiplier tables (x10000) ── sector 0 ──────────────

pub const N_S0_R8:  [u64;  9] = [56000, 21000, 11000, 9775, 5000, 9775, 11000, 21000, 56000];
pub const N_S0_R9:  [u64; 10] = [56000, 20000, 16000, 10000, 6800, 6800, 10000, 16000, 20000, 56000];
pub const N_S0_R10: [u64; 11] = [89000, 30000, 14000, 11000, 9755, 5000, 9755, 11000, 14000, 30000, 89000];
pub const N_S0_R11: [u64; 12] = [85000, 30000, 19000, 13000, 9685, 7000, 7000, 9685, 13000, 19000, 30000, 85000];
pub const N_S0_R12: [u64; 13] = [108000, 30000, 16000, 14000, 11000, 9736, 5000, 9736, 11000, 14000, 16000, 30000, 108000];
pub const N_S0_R13: [u64; 14] = [89000, 40000, 30000, 19000, 12000, 9000, 6756, 6756, 9000, 12000, 19000, 30000, 40000, 89000];
pub const N_S0_R14: [u64; 15] = [87000, 40000, 19000, 14000, 13000, 11000, 9721, 5000, 9721, 11000, 13000, 14000, 19000, 40000, 87000];
pub const N_S0_R15: [u64; 16] = [177000, 80000, 30000, 20000, 15000, 11000, 9666, 7000, 7000, 9666, 11000, 15000, 20000, 30000, 80000, 177000];
pub const N_S0_R16: [u64; 17] = [230000, 90000, 20000, 14000, 14000, 12000, 11000, 9707, 5000, 9707, 11000, 12000, 14000, 14000, 20000, 90000, 230000];

// ── Normal multiplier tables (x10000) ── sector 1 ──────────────

pub const N_S1_R8:  [u64;  9] = [131000, 30000, 13000, 6775, 4000, 6775, 13000, 30000, 131000];
pub const N_S1_R9:  [u64; 10] = [176000, 40000, 17000, 8700, 5000, 5000, 8700, 17000, 40000, 176000];
pub const N_S1_R10: [u64; 11] = [225000, 50000, 20000, 13571, 6000, 4000, 6000, 13571, 20000, 50000, 225000];
pub const N_S1_R11: [u64; 12] = [240000, 60000, 30000, 17364, 7000, 5000, 5000, 7000, 17364, 30000, 60000, 240000];
pub const N_S1_R12: [u64; 13] = [336000, 110000, 40000, 20000, 10578, 6000, 3000, 6000, 10578, 20000, 40000, 110000, 336000];
pub const N_S1_R13: [u64; 14] = [441000, 130000, 60000, 30000, 12415, 7000, 4000, 4000, 7000, 12415, 30000, 60000, 130000, 441000];
pub const N_S1_R14: [u64; 15] = [602000, 150000, 70000, 40000, 19000, 9582, 5000, 2000, 5000, 9582, 19000, 40000, 70000, 150000, 602000];
pub const N_S1_R15: [u64; 16] = [902000, 180000, 110000, 50000, 28785, 13000, 5000, 3000, 3000, 5000, 13000, 28785, 50000, 110000, 180000, 902000];
pub const N_S1_R16: [u64; 17] = [1204000, 410000, 100000, 50000, 30000, 15000, 9582, 5000, 3000, 5000, 9582, 15000, 30000, 50000, 100000, 410000, 1204000];

// ── Normal multiplier tables (x10000) ── sector 2 ──────────────

pub const N_S2_R8:  [u64;  9] = [289000, 40000, 14550, 3000, 2000, 3000, 14550, 40000, 289000];
pub const N_S2_R9:  [u64; 10] = [428000, 70000, 19300, 6000, 2000, 2000, 6000, 19300, 70000, 428000];
pub const N_S2_R10: [u64; 11] = [757000, 100000, 28857, 9000, 3000, 2000, 3000, 9000, 28857, 100000, 757000];
pub const N_S2_R11: [u64; 12] = [1185000, 140000, 50112, 14000, 4000, 2000, 2000, 4000, 14000, 50112, 140000, 1185000];
pub const N_S2_R12: [u64; 13] = [1681000, 240000, 77824, 20000, 7000, 2000, 2000, 2000, 7000, 20000, 77824, 240000, 1681000];
pub const N_S2_R13: [u64; 14] = [2574000, 370000, 110000, 38534, 10000, 2000, 2000, 2000, 2000, 10000, 38534, 110000, 370000, 2574000];
pub const N_S2_R14: [u64; 15] = [4234000, 560000, 180000, 50000, 18165, 3000, 2000, 2000, 2000, 3000, 18165, 50000, 180000, 560000, 4234000];
pub const N_S2_R15: [u64; 16] = [6189000, 830000, 270000, 80000, 28775, 5000, 2000, 2000, 2000, 2000, 5000, 28775, 80000, 270000, 830000, 6189000];
pub const N_S2_R16: [u64; 17] = [10140000, 1300000, 260000, 90000, 40000, 19235, 2000, 2000, 2000, 2000, 2000, 19235, 40000, 90000, 260000, 1300000, 10140000];

// ── Extreme multiplier tables (x10000) ─────────────────────────

pub const E_S0_R12: [u64; 13] = [10000000, 500000, 14000, 5000, 2000, 1000, 1000, 1000, 2000, 5000, 14000, 500000, 10000000];
pub const E_S1_R14: [u64; 15] = [35000000, 2080000, 9000, 4000, 3000, 2000, 2000, 1000, 2000, 2000, 3000, 4000, 9000, 2080000, 35000000];
pub const E_S2_R16: [u64; 17] = [100000000, 5300000, 150000, 60000, 15000, 6000, 2000, 1000, 1000, 1000, 2000, 6000, 15000, 60000, 150000, 5300000, 100000000];

/// Get the multiplier row for the given (sector, rows, is_extreme) combination.
/// Returns `None` for invalid combinations.
pub fn get_multiplier_row(sector: u8, rows: u32, is_extreme: bool) -> Option<&'static [u64]> {
    if is_extreme {
        return match (sector, rows) {
            (0, 12) => Some(&E_S0_R12),
            (1, 14) => Some(&E_S1_R14),
            (2, 16) => Some(&E_S2_R16),
            _ => None,
        };
    }
    match (sector, rows) {
        (0,  8) => Some(&N_S0_R8),
        (0,  9) => Some(&N_S0_R9),
        (0, 10) => Some(&N_S0_R10),
        (0, 11) => Some(&N_S0_R11),
        (0, 12) => Some(&N_S0_R12),
        (0, 13) => Some(&N_S0_R13),
        (0, 14) => Some(&N_S0_R14),
        (0, 15) => Some(&N_S0_R15),
        (0, 16) => Some(&N_S0_R16),
        (1,  8) => Some(&N_S1_R8),
        (1,  9) => Some(&N_S1_R9),
        (1, 10) => Some(&N_S1_R10),
        (1, 11) => Some(&N_S1_R11),
        (1, 12) => Some(&N_S1_R12),
        (1, 13) => Some(&N_S1_R13),
        (1, 14) => Some(&N_S1_R14),
        (1, 15) => Some(&N_S1_R15),
        (1, 16) => Some(&N_S1_R16),
        (2,  8) => Some(&N_S2_R8),
        (2,  9) => Some(&N_S2_R9),
        (2, 10) => Some(&N_S2_R10),
        (2, 11) => Some(&N_S2_R11),
        (2, 12) => Some(&N_S2_R12),
        (2, 13) => Some(&N_S2_R13),
        (2, 14) => Some(&N_S2_R14),
        (2, 15) => Some(&N_S2_R15),
        (2, 16) => Some(&N_S2_R16),
        _ => None,
    }
}

/// Check whether a (sector, rows, is_extreme) triple is a valid Plinko config.
pub fn is_valid_config(sector: u8, rows: u32, is_extreme: bool) -> bool {
    get_multiplier_row(sector, rows, is_extreme).is_some()
}

/// Extract the Plinko path from the Poseidon2 random output.
///
/// Each bit of `random[0]` (from LSB upward) determines the ball direction
/// at each row: 0 = left, 1 = right.
pub fn path_from_random(random: &[u64; 4], rows: u32) -> Vec<u8> {
    assert!(rows >= MIN_ROWS && rows <= MAX_ROWS);
    let r0 = random[0];
    (0..rows).map(|i| ((r0 >> i) & 1) as u8).collect()
}

/// Bucket index = number of rightward moves (popcount of path bits).
pub fn bucket_index(path: &[u8]) -> u32 {
    path.iter().map(|&b| b as u32).sum()
}

/// Bucket index directly from random, without allocating path vec.
pub fn bucket_index_from_random(random: &[u64; 4], rows: u32) -> u32 {
    assert!(rows >= MIN_ROWS && rows <= MAX_ROWS);
    let mask = (1u64 << rows) - 1;
    (random[0] & mask).count_ones()
}

/// Look up the multiplier (x10000) for a given config and bucket.
/// Panics on invalid (sector, rows, is_extreme) or out-of-range bucket.
pub fn lookup_multiplier(sector: u8, rows: u32, is_extreme: bool, bucket: u32) -> u64 {
    let row = get_multiplier_row(sector, rows, is_extreme)
        .unwrap_or_else(|| panic!(
            "invalid plinko config: sector={sector}, rows={rows}, is_extreme={is_extreme}"
        ));
    assert!(
        (bucket as usize) < row.len(),
        "bucket {bucket} out of range for rows={rows} (max={})",
        row.len() - 1
    );
    row[bucket as usize]
}

/// Full payout computation for Plinko — pure integer arithmetic, zero floats.
///
/// Unlike Dice, Plinko ALWAYS pays out (even sub-1x multipliers), so
/// `win_amount` is never forced to zero by a lose condition.
/// The `is_win` flag is cosmetic: true when multiplier >= 1.0x.
pub fn compute_payout(
    random: &[u64; 4],
    bet_atomic: u64,
    sector: u8,
    rows: u32,
    is_extreme: bool,
) -> GamePayout {
    let bucket = bucket_index_from_random(random, rows);
    let multi = lookup_multiplier(sector, rows, is_extreme, bucket);

    let raw_win = (bet_atomic as u128 * multi as u128) / PAYOUT_DIVISOR as u128;
    let win_amount = (raw_win as u64).min(MAX_WIN);

    GamePayout {
        win_amount,
        roll_number: bucket,
        is_win: multi >= PAYOUT_DIVISOR,
        multiplier: multi,
    }
}

/// Compute RTP × 100 for a given Plinko config (e.g. 9799 = 97.99%).
///
/// Plinko bucket probability follows binomial distribution:
///   P(bucket=k) = C(rows, k) / 2^rows
///
/// RTP = E[multiplier] / PAYOUT_DIVISOR = sum(C(rows,k) * multi[k]) / (2^rows * 10000)
pub fn compute_rtp_x100(sector: u8, rows: u32, is_extreme: bool) -> Option<u64> {
    let table = get_multiplier_row(sector, rows, is_extreme)?;

    let mut weighted_sum: u128 = 0;
    let mut c: u128 = 1;
    for k in 0..=rows as usize {
        weighted_sum += c * table[k] as u128;
        if k < rows as usize {
            c = c * (rows as u128 - k as u128) / (k as u128 + 1);
        }
    }

    Some((weighted_sum / (1u128 << rows)) as u64)
}

/// Collect all 30 multiplier rows (normal 27 + extreme 3) in a deterministic order.
/// Used by the circuit to pre-compute table commitments.
/// Order: normal tables first (sector 0 rows 8..16, sector 1 rows 8..16, sector 2 rows 8..16),
/// then extreme tables (sector 0, 1, 2).
pub fn all_multiplier_rows() -> Vec<(u8, u32, bool, &'static [u64])> {
    let mut rows = Vec::with_capacity(30);
    for sector in 0..3u8 {
        for r in MIN_ROWS..=MAX_ROWS {
            if let Some(table) = get_multiplier_row(sector, r, false) {
                rows.push((sector, r, false, table));
            }
        }
    }
    for sector in 0..3u8 {
        let extreme_rows = match sector {
            0 => 12,
            1 => 14,
            2 => 16,
            _ => unreachable!(),
        };
        let table = get_multiplier_row(sector, extreme_rows, true).unwrap();
        rows.push((sector, extreme_rows, true, table));
    }
    rows
}
