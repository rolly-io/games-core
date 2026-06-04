use crate::shared::{GamePayout, MAX_WIN, PAYOUT_DIVISOR};

pub const KENO_GAME_ID: u8 = 3;
pub const NUMBERS_RANGE: usize = 40;
pub const DRAW_COUNT: usize = 10;
pub const MAX_PICKS: usize = 10;
/// C(40, 10) = 847_660_528 — total number of possible 10-element subsets of {0..39}.
pub const COMBO_40_10: u64 = 847_660_528;

// ── Binomial coefficient table ──────────────────────────────────
// BINOM[n][k] = C(n, k) for n in [0, 39], k in [0, 10].

const fn build_binom_table() -> [[u64; 11]; 40] {
    let mut table = [[0u64; 11]; 40];
    let mut n = 0usize;
    while n < 40 {
        table[n][0] = 1;
        let mut k = 1usize;
        while k <= 10 && k <= n {
            table[n][k] = table[n - 1][k - 1] + table[n - 1][k];
            k += 1;
        }
        n += 1;
    }
    table
}

pub const BINOM: [[u64; 11]; 40] = build_binom_table();

#[inline]
pub fn binom(n: u32, k: u32) -> u64 {
    if n >= 40 || k > 10 || k > n {
        0
    } else {
        BINOM[n as usize][k as usize]
    }
}

// ── Low risk multiplier tables (x10000) ─────────────────────────

pub const L_P1:  [u64;  2] = [7000, 18200];
pub const L_P2:  [u64;  3] = [0, 20000, 36500];
pub const L_P3:  [u64;  4] = [0, 11000, 13000, 260100];
pub const L_P4:  [u64;  5] = [0, 0, 21500, 79200, 900000];
pub const L_P5:  [u64;  6] = [0, 0, 14700, 41500, 131200, 3000000];
pub const L_P6:  [u64;  7] = [0, 0, 11000, 19600, 60000, 1000000, 7000000];
pub const L_P7:  [u64;  8] = [0, 0, 11000, 15500, 34700, 150000, 2270000, 7000000];
pub const L_P8:  [u64;  9] = [0, 0, 11000, 14600, 20000, 55000, 380000, 1000000, 8000000];
pub const L_P9:  [u64; 10] = [0, 0, 11000, 13000, 16000, 25000, 73500, 500000, 2500000, 10000000];
pub const L_P10: [u64; 11] = [0, 0, 11000, 12000, 12500, 18000, 34500, 130000, 500000, 2500000, 10000000];

// ── Medium risk multiplier tables (x10000) ──────────────────────

pub const M_P1:  [u64;  2] = [4000, 27200];
pub const M_P2:  [u64;  3] = [0, 17500, 53000];
pub const M_P3:  [u64;  4] = [0, 0, 28000, 500000];
pub const M_P4:  [u64;  5] = [0, 0, 17000, 98000, 1000000];
pub const M_P5:  [u64;  6] = [0, 0, 14000, 40000, 136100, 3750000];
pub const M_P6:  [u64;  7] = [0, 0, 0, 30000, 90000, 1760000, 7010000];
pub const M_P7:  [u64;  8] = [0, 0, 0, 20000, 70000, 280000, 4000000, 8000000];
pub const M_P8:  [u64;  9] = [0, 0, 0, 20000, 40000, 105000, 650000, 3950000, 9000000];
pub const M_P9:  [u64; 10] = [0, 0, 0, 20000, 24000, 50000, 150000, 960000, 5000000, 10000000];
pub const M_P10: [u64; 11] = [0, 0, 0, 16000, 20000, 40000, 60000, 210000, 1000000, 5000000, 10000000];

// ── High risk multiplier tables (x10000) ────────────────────────

pub const H_P1:  [u64;  2] = [0, 39200];
pub const H_P2:  [u64;  3] = [0, 0, 169800];
pub const H_P3:  [u64;  4] = [0, 0, 0, 806900];
pub const H_P4:  [u64;  5] = [0, 0, 0, 100000, 2550000];
pub const H_P5:  [u64;  6] = [0, 0, 0, 45000, 470000, 4500000];
pub const H_P6:  [u64;  7] = [0, 0, 0, 0, 110000, 3450000, 7000000];
pub const H_P7:  [u64;  8] = [0, 0, 0, 0, 70000, 885000, 4000000, 8000000];
pub const H_P8:  [u64;  9] = [0, 0, 0, 0, 50000, 200000, 2600000, 6130000, 9000000];
pub const H_P9:  [u64; 10] = [0, 0, 0, 0, 40000, 110000, 560000, 4680000, 8000000, 10000000];
pub const H_P10: [u64; 11] = [0, 0, 0, 0, 35000, 80000, 120000, 570000, 5000000, 8000000, 10000000];

/// Get the multiplier table for a (risk, pick_count) combination.
/// `risk`: 0=low, 1=medium, 2=high.  `pick_count`: 1..10.
/// Returns `None` for invalid inputs.
pub fn get_multiplier_table(risk: u8, pick_count: u8) -> Option<&'static [u64]> {
    match (risk, pick_count) {
        (0,  1) => Some(&L_P1),
        (0,  2) => Some(&L_P2),
        (0,  3) => Some(&L_P3),
        (0,  4) => Some(&L_P4),
        (0,  5) => Some(&L_P5),
        (0,  6) => Some(&L_P6),
        (0,  7) => Some(&L_P7),
        (0,  8) => Some(&L_P8),
        (0,  9) => Some(&L_P9),
        (0, 10) => Some(&L_P10),
        (1,  1) => Some(&M_P1),
        (1,  2) => Some(&M_P2),
        (1,  3) => Some(&M_P3),
        (1,  4) => Some(&M_P4),
        (1,  5) => Some(&M_P5),
        (1,  6) => Some(&M_P6),
        (1,  7) => Some(&M_P7),
        (1,  8) => Some(&M_P8),
        (1,  9) => Some(&M_P9),
        (1, 10) => Some(&M_P10),
        (2,  1) => Some(&H_P1),
        (2,  2) => Some(&H_P2),
        (2,  3) => Some(&H_P3),
        (2,  4) => Some(&H_P4),
        (2,  5) => Some(&H_P5),
        (2,  6) => Some(&H_P6),
        (2,  7) => Some(&H_P7),
        (2,  8) => Some(&H_P8),
        (2,  9) => Some(&H_P9),
        (2, 10) => Some(&H_P10),
        _ => None,
    }
}

// ── Combinatorial Number System ─────────────────────────────────

/// Compute the rank of a sorted combination in the combinatorial number system.
/// `drawn` must be sorted ascending with unique values in [0, 39].
pub fn rank_combination(drawn: &[u8; DRAW_COUNT]) -> u64 {
    let mut rank = 0u64;
    for i in 0..DRAW_COUNT {
        rank += binom(drawn[i] as u32, (i + 1) as u32);
    }
    rank
}

/// Unrank a combinatorial number to a sorted 10-element subset of {0..39}.
pub fn unrank_combination(mut rank: u64) -> [u8; DRAW_COUNT] {
    assert!(rank < COMBO_40_10, "rank {rank} >= C(40,10) = {COMBO_40_10}");
    let mut result = [0u8; DRAW_COUNT];
    for i in (1..=DRAW_COUNT as u32).rev() {
        let mut c = NUMBERS_RANGE as u32 - 1;
        while binom(c, i) > rank {
            c -= 1;
        }
        result[(i - 1) as usize] = c as u8;
        rank -= binom(c, i);
    }
    result
}

/// Extract 10 unique drawn numbers from Poseidon2 random output.
///
/// `combo_index = random[0] % C(40,10)`, then unranked to a sorted subset.
pub fn drawn_from_random(random: &[u64; 4]) -> [u8; DRAW_COUNT] {
    let combo_index = random[0] % COMBO_40_10;
    unrank_combination(combo_index)
}

/// Count how many of the player's selected numbers appear in the drawn set.
pub fn count_matches(selected: &[u8], drawn: &[u8; DRAW_COUNT]) -> u8 {
    selected.iter().filter(|s| drawn.contains(s)).count() as u8
}

/// Full payout computation for Keno — pure integer arithmetic, zero floats.
///
/// `random`: 4 Goldilocks field elements (Poseidon2 output).
/// `bet_atomic`: bet in atomic units (1 USDT = 1_000_000).
/// `risk`: 0=low, 1=medium, 2=high.
/// `selected`: player-chosen numbers, 0-indexed [0..39], 1 to 10 unique values.
pub fn compute_payout(
    random: &[u64; 4],
    bet_atomic: u64,
    risk: u8,
    selected: &[u8],
) -> GamePayout {
    let pick_count = selected.len();
    assert!(
        pick_count >= 1 && pick_count <= MAX_PICKS,
        "pick_count must be 1..{MAX_PICKS}, got {pick_count}"
    );
    assert!(risk <= 2, "risk must be 0..2, got {risk}");

    {
        let mut sorted = selected.to_vec();
        sorted.sort();
        for (i, &s) in sorted.iter().enumerate() {
            assert!(
                (s as usize) < NUMBERS_RANGE,
                "selected number {s} out of range [0, {NUMBERS_RANGE})"
            );
            if i > 0 {
                assert!(s > sorted[i - 1], "selected contains duplicates");
            }
        }
    }

    let drawn = drawn_from_random(random);
    let matches = count_matches(selected, &drawn);

    let table = get_multiplier_table(risk, pick_count as u8)
        .expect("invalid risk/pick_count combination");
    let multi = table[matches as usize];

    let win_amount = if multi > 0 {
        let raw = (bet_atomic as u128 * multi as u128) / PAYOUT_DIVISOR as u128;
        (raw as u64).min(MAX_WIN)
    } else {
        0
    };

    GamePayout {
        win_amount,
        roll_number: matches as u32,
        is_win: multi > 0,
        multiplier: multi,
    }
}

/// Compute RTP × 100 for a given Keno config (e.g. 9800 = 98.00%).
///
/// Keno match probability follows hypergeometric distribution:
///   P(m matches) = C(pick,m) * C(40-pick, 10-m) / C(40, 10)
///
/// RTP = sum(P(m) * multi[m]) / PAYOUT_DIVISOR
pub fn compute_rtp_x100(risk: u8, pick_count: u8) -> Option<u64> {
    let table = get_multiplier_table(risk, pick_count)?;
    let pc = pick_count as u32;
    let complement = NUMBERS_RANGE as u32 - pc;

    let mut weighted_sum: u128 = 0;
    for m in 0..=pc.min(DRAW_COUNT as u32) {
        let draw_remaining = DRAW_COUNT as u32 - m;
        if draw_remaining > complement {
            continue;
        }
        let prob_num = binom(pc, m) as u128 * binom(complement, draw_remaining) as u128;
        weighted_sum += prob_num * table[m as usize] as u128;
    }

    Some((weighted_sum / COMBO_40_10 as u128) as u64)
}

/// Collect all 30 multiplier rows in deterministic order for circuit commitments.
/// Order: risk 0 picks 1..10, risk 1 picks 1..10, risk 2 picks 1..10.
pub fn all_multiplier_rows() -> Vec<(u8, u8, &'static [u64])> {
    let mut rows = Vec::with_capacity(30);
    for risk in 0..3u8 {
        for picks in 1..=10u8 {
            if let Some(table) = get_multiplier_table(risk, picks) {
                rows.push((risk, picks, table));
            }
        }
    }
    rows
}
