/// 1 USDT = 1_000_000 atomic units (6 decimals, same as on-chain USDT).
pub const USDT_DECIMALS: u64 = 1_000_000;

/// All multiplier values are scaled ×10000 (e.g. 19800 = 1.9800x).
pub const PAYOUT_DIVISOR: u64 = 10_000;

/// Maximum win amount per bet in atomic units (10 000 USDT).
pub const MAX_WIN: u64 = 10_000 * USDT_DECIMALS;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameId {
    Limbo = 1,
    Dice = 2,
    Keno = 3,
    Plinko = 4,
    Coinflip = 5,
    Crash = 6,
}

/// Result of a payout computation for any game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GamePayout {
    /// Win amount in atomic units (divide by USDT_DECIMALS to get USDT).
    /// Zero when the player loses.
    pub win_amount: u64,
    /// Raw roll number produced by the RNG (game-specific range).
    pub roll_number: u32,
    /// Whether the player won this round.
    pub is_win: bool,
    /// Multiplier scaled x10000 (e.g. 19800 = 1.9800x).
    pub multiplier: u64,
}
