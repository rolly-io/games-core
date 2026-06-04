use crate::shared::{GamePayout, MAX_WIN, PAYOUT_DIVISOR};

pub const LIMBO_GAME_ID: u8 = 1;
pub const POW2_32: u64 = 4_294_967_296;
pub const RTP_PERCENT: u32 = 99;
pub const MIN_MULTI_X100: u64 = 101;
pub const MAX_MULTI_X100: u64 = 999_999;
/// Circuit-side payout divisor: `bet × prediction_x100 / 100`.
/// Equivalent to `bet × prediction_x10000 / PAYOUT_DIVISOR` in game-core.
pub const LIMBO_PAYOUT_DIVISOR: u64 = PAYOUT_DIVISOR / 100;

/// Compute the actual multiplier x100 from Poseidon2 random output via inverse transform sampling.
///
///   v     = random[0] & 0xFFFF_FFFF  (lower 32 bits)
///   denom = 2^32 - v                 (range 1..=2^32)
///   raw   = floor(RTP_PERCENT * 2^32 / denom)
///   result = clamp(raw, 101, 999999)
pub fn multiplier_from_random(random: &[u64; 4]) -> u64 {
    let v = random[0] & 0xFFFF_FFFF;
    let denom = POW2_32 - v;
    let raw = (RTP_PERCENT as u128 * POW2_32 as u128) / denom as u128;
    (raw as u64).clamp(MIN_MULTI_X100, MAX_MULTI_X100)
}

/// Full payout computation for Limbo — pure integer arithmetic, zero floats.
///
/// Win condition (circuit-equivalent, avoids division):
///   `prediction_x100 * denom <= RTP_PERCENT * 2^32`
///
/// Payout on win: `min(bet * prediction_x10000 / 10_000, MAX_WIN)`
/// where `prediction_x10000 = prediction_x100 * 100`.
pub fn compute_payout(
    random: &[u64; 4],
    bet_atomic: u64,
    prediction_x100: u32,
) -> GamePayout {
    assert!(
        prediction_x100 >= MIN_MULTI_X100 as u32 && prediction_x100 <= MAX_MULTI_X100 as u32,
        "prediction_x100 must be in [{MIN_MULTI_X100}, {MAX_MULTI_X100}], got {prediction_x100}"
    );

    let v = random[0] & 0xFFFF_FFFF;
    let denom = POW2_32 - v;
    let multi_x100 = multiplier_from_random(random);

    let lhs = prediction_x100 as u128 * denom as u128;
    let rhs = RTP_PERCENT as u128 * POW2_32 as u128;
    let won = lhs <= rhs;

    let prediction_x10000 = prediction_x100 as u64 * 100;
    let win_amount = if won {
        let raw = (bet_atomic as u128 * prediction_x10000 as u128) / PAYOUT_DIVISOR as u128;
        (raw as u64).min(MAX_WIN)
    } else {
        0
    };

    GamePayout {
        win_amount,
        roll_number: multi_x100 as u32,
        is_win: won,
        multiplier: if won { prediction_x10000 } else { 0 },
    }
}
