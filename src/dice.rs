use crate::shared::{GamePayout, MAX_WIN, PAYOUT_DIVISOR};

pub const DICE_GAME_ID: u8 = 2;
pub const TOTAL_RANGE: u32 = 1000;
pub const RTP_PERCENT: u32 = 99;
pub const MIN_RANGE: u32 = 1;
pub const MAX_RANGE: u32 = 950;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiceMode {
    Under = 0,
    Over = 1,
    In = 2,
    Out = 3,
}

impl DiceMode {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Under),
            1 => Some(Self::Over),
            2 => Some(Self::In),
            3 => Some(Self::Out),
            _ => None,
        }
    }
}

/// Extract a roll number [0, 1000) from Poseidon2 random output.
///
/// Modulo bias: Goldilocks p ~ 2^64, p % 1000 = 321.
/// Values 0..320 appear with probability (floor(p/1000)+1)/p,
/// values 321..999 with floor(p/1000)/p. Delta ~ 5.4e-20 per outcome — negligible.
pub fn roll_number_from_random(random: &[u64; 4]) -> u32 {
    (random[0] % TOTAL_RANGE as u64) as u32
}

/// Determine win/loss for a given mode, prediction range, and roll.
pub fn is_win(mode: DiceMode, prediction_range: [u32; 2], roll: u32) -> bool {
    match mode {
        DiceMode::Under => roll < prediction_range[0],
        DiceMode::Over => roll > prediction_range[0],
        DiceMode::In => prediction_range[0] <= roll && roll <= prediction_range[1],
        DiceMode::Out => roll < prediction_range[0] || roll > prediction_range[1],
    }
}

/// Count how many roll outcomes produce a win for the given mode/prediction.
pub fn win_numbers(mode: DiceMode, prediction_range: [u32; 2]) -> u32 {
    match mode {
        DiceMode::Under => prediction_range[0],
        DiceMode::Over => TOTAL_RANGE - 1 - prediction_range[0],
        DiceMode::In => prediction_range[1] + 1 - prediction_range[0],
        DiceMode::Out => TOTAL_RANGE - (prediction_range[1] + 1 - prediction_range[0]),
    }
}

/// Compute multiplier x10000 via integer division (always floors — favours house).
///
/// Formula: (TOTAL_RANGE x RTP_PERCENT x 100) / wn = 9_900_000 / wn
pub fn multiplier(wn: u32) -> u64 {
    assert!(
        wn >= MIN_RANGE && wn <= MAX_RANGE,
        "win_numbers out of valid range [{}, {}]: got {}",
        MIN_RANGE,
        MAX_RANGE,
        wn,
    );
    let numerator = TOTAL_RANGE as u128 * RTP_PERCENT as u128 * 100;
    (numerator / wn as u128) as u64
}

/// Full payout computation for Dice — pure integer arithmetic, zero floats.
pub fn compute_payout(
    random: &[u64; 4],
    bet_atomic: u64,
    mode: DiceMode,
    prediction_range: [u32; 2],
) -> GamePayout {
    let roll = roll_number_from_random(random);
    let won = is_win(mode, prediction_range, roll);
    let wn = win_numbers(mode, prediction_range);
    let multi = multiplier(wn);

    let win_amount = if won {
        let raw = (bet_atomic as u128 * multi as u128) / PAYOUT_DIVISOR as u128;
        (raw as u64).min(MAX_WIN)
    } else {
        0
    };

    GamePayout {
        win_amount,
        roll_number: roll,
        is_win: won,
        multiplier: multi,
    }
}
