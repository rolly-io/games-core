use crate::shared::{GamePayout, MAX_WIN};

pub const AVIATO_GAME_ID: u8 = 6;
pub const POW2_32: u64 = 4_294_967_296;
pub const RTP_PERCENT: u32 = 99;
/// Minimum aviato point (×100). Aviato CAN land at 1.00x — everyone loses.
pub const MIN_AVIATO_X100: u64 = 100;
/// Minimum valid cashout multiplier (×100). Users can stop at 1.01x earliest.
pub const MIN_CASHOUT_X100: u64 = 101;
pub const MAX_MULTI_X100: u64 = 1_000_000;
/// Aviato payout divisor: cashout_x100 / 100 (not ×10000 like other games).
pub const AVIATO_PAYOUT_DIVISOR: u64 = 100;

/// Compute the aviato point (×100) from Poseidon2 random output via inverse transform sampling.
///
///   v_lo  = random[0] & 0xFFFF_FFFF  (lower 32 bits)
///   denom = 2^32 - v_lo              (range 1..=2^32)
///   raw   = floor(RTP_PERCENT × 2^32 / denom)
///   result = clamp(raw, MIN_AVIATO_X100, MAX_MULTI_X100)
///
/// When raw < MIN_AVIATO_X100 (≈1% of rounds), aviato point = 1.00x and all
/// users lose because minimum cashout is 1.01x.
pub fn aviato_point_from_random(random: &[u64; 4]) -> u64 {
    let v_lo = random[0] & 0xFFFF_FFFF;
    let denom = POW2_32 - v_lo;
    let raw = (RTP_PERCENT as u128 * POW2_32 as u128) / denom as u128;
    (raw as u64).clamp(MIN_AVIATO_X100, MAX_MULTI_X100)
}

/// Full payout computation for Aviato — pure integer arithmetic, zero floats.
///
/// `random`: 4 Goldilocks field elements (Poseidon2 output of server_seed).
/// `bet_atomic`: bet in atomic units (1 USDT = 1_000_000).
/// `cashout_x100`: multiplier at which the user cashed out (×100).
///   - 0 = user did NOT cash out (loss — rode the rocket until it crashed).
///   - 101..=MAX_MULTI_X100 = user stopped at this multiplier (min 1.01x).
///
/// Win condition: `cashout_x100 > 0 && cashout_x100 <= aviato_point`.
/// Payout on win: `min(bet × cashout_x100 / AVIATO_PAYOUT_DIVISOR, MAX_WIN)`.
pub fn compute_payout(
    random: &[u64; 4],
    bet_atomic: u64,
    cashout_x100: u32,
) -> GamePayout {
    assert!(
        cashout_x100 == 0
            || (cashout_x100 >= MIN_CASHOUT_X100 as u32
                && cashout_x100 <= MAX_MULTI_X100 as u32),
        "cashout_x100 must be 0 (no cashout) or in [{MIN_CASHOUT_X100}, {MAX_MULTI_X100}], got {cashout_x100}"
    );

    let aviato_x100 = aviato_point_from_random(random);

    let won = cashout_x100 > 0 && (cashout_x100 as u64) <= aviato_x100;

    let (win_amount, multiplier) = if won {
        let raw = (bet_atomic as u128 * cashout_x100 as u128) / AVIATO_PAYOUT_DIVISOR as u128;
        let capped = (raw as u64).min(MAX_WIN);
        (capped, cashout_x100 as u64 * 100)
    } else {
        (0, 0)
    };

    GamePayout {
        win_amount,
        roll_number: aviato_x100 as u32,
        is_win: won,
        multiplier,
    }
}
