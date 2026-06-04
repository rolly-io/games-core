use crate::shared::{GamePayout, MAX_WIN, PAYOUT_DIVISOR};

pub const COINFLIP_GAME_ID: u8 = 5;

/// House edge 1% → RTP 99%.
pub const RTP_PERCENT: u32 = 99;
/// Payout multiplier ×10000, derived: 2 × RTP% × 100 = 19_800 (1.98×).
pub const MULTIPLIER: u64 = RTP_PERCENT as u64 * 200;

/// Extract the coinflip roll (0 or 1) from Poseidon2 random output.
///
/// `random[0] % 2` — uniform because Goldilocks p - 1 is even.
pub fn roll_from_random(random: &[u64; 4]) -> u32 {
    (random[0] % 2) as u32
}

/// Full payout computation for CoinFlip — pure integer arithmetic, zero floats.
///
/// `prediction` must be 0 or 1.
/// Win condition: `roll == prediction`.
pub fn compute_payout(
    random: &[u64; 4],
    bet_atomic: u64,
    prediction: u8,
) -> GamePayout {
    assert!(prediction <= 1, "prediction must be 0 or 1, got {prediction}");

    let roll = roll_from_random(random);
    let won = roll == prediction as u32;

    let win_amount = if won {
        let raw = (bet_atomic as u128 * MULTIPLIER as u128) / PAYOUT_DIVISOR as u128;
        (raw as u64).min(MAX_WIN)
    } else {
        0
    };

    GamePayout {
        win_amount,
        roll_number: roll,
        is_win: won,
        multiplier: if won { MULTIPLIER } else { 0 },
    }
}
