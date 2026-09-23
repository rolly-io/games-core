//! Blackjack payout logic — pure `u64` arithmetic, zero floats, no hashing.
//!
//! This is a faithful port of the legacy JS engine
//! (`rolly-backend/games/Blackjack/GameEngine.js`:
//! `getHandPoints` / `calculateRound` / `closeRound` / `applyRoundAction` /
//! `fillRound`) driven exactly the way `slowBet()` orchestrates one round:
//! deal → per-action `calculateRound` → `closeRound` + final `calculateRound`.
//!
//! Poseidon2, the fair-shoe shuffle and every hash live in the circuit and the
//! wasm-signer. This crate only sees the already-dealt cards (as rank codes)
//! and the encoded action sequence, and reproduces the resulting win amount.
//!
//! ## Card encoding (rank codes, suits irrelevant to points)
//!
//! | code | rank        | points |
//! |------|-------------|--------|
//! | 0    | Ace         | 1 / 11 |
//! | 1..8 | 2..9        | 2..9   |
//! | 9    | 10          | 10     |
//! | 10   | Jack        | 10     |
//! | 11   | Queen       | 10     |
//! | 12   | King        | 10     |
//!
//! Deck layout mirrors the JS cursor (`getNextCard`): indices 0,1 = player's
//! first two cards, index 2 = dealer up-card, index 3 = dealer hole card,
//! indices 4.. = subsequent draws. The dealer hole (index 3) is reserved even
//! before it is revealed, because the cursor uses `max(dealer_len, 2)`.

use crate::shared::{GamePayout, MAX_WIN};

/// game_id field value for Blackjack (Limbo=1 .. Crash=6, Blackjack=7).
pub const BLACKJACK_GAME_ID: u8 = 7;

/// Player/dealer bust threshold.
pub const POINTS_LIMIT: u32 = 21;
/// Dealer must keep drawing while below this.
pub const DEALER_POINTS_LIMIT: u32 = 17;

/// Upper bound on the number of cards a single round can consume (2 hands ×
/// up to 10 cards each + dealer draws). The circuit uses the same fixed bound.
pub const MAX_CARDS: usize = 30;

// ── Action codes (encoded in `actions_packed`) ──────────────────────────────
pub const ACTION_HIT: u8 = 1;
pub const ACTION_STAND: u8 = 2;
pub const ACTION_DOUBLE: u8 = 3;
pub const ACTION_SPLIT: u8 = 4;
pub const ACTION_INSURANCE_ACCEPT: u8 = 5;
pub const ACTION_INSURANCE_DECLINE: u8 = 6;

// ── Shoe layout + fair shuffle (RNG supplied by the caller) ──────────────────
//
// game-core stays hash-free: exactly like every other game receives its
// `random` already hashed, the shoe shuffle receives the per-swap randomness
// pre-computed with Poseidon2 by the caller (the circuit in-circuit, the
// tx-validator / witness engine natively). Only the deterministic layout, swap
// application and action (un)packing live here, so all layers share one
// definition of the game logic.

/// Number of standard 52-card decks the shoe is built from.
pub const DECK_COUNT: usize = 8;
/// Physical shoe size: `DECK_COUNT × 52` = 416 cards.
pub const SHOE_SIZE: usize = DECK_COUNT * 52;

/// Maximum number of player actions per round (fixed unroll bound). Packed
/// base-8 into a single field element, so `3 × MAX_ACTIONS < 63`.
pub const MAX_ACTIONS: usize = 20;
/// Base used to pack the action sequence: `packed = Σ action[i] · 8^i`.
pub const ACTION_PACK_BASE: u64 = 8;

/// Rank code of shoe position `i` in the *unshuffled* layout. Every 52-card
/// block is `rank·4 + suit`, so `code = (i mod 52) / 4` — 32 copies of each of
/// the 13 ranks across the 8 decks. Only the rank (points value) matters;
/// suits are cosmetic and restored off-engine.
#[inline]
pub fn shoe_layout_code(i: usize) -> u8 {
    ((i % 52) / 4) as u8
}

/// Cosmetic suit code (0..=3) of shoe position `i` in the *unshuffled* layout:
/// every 52-card block is `rank·4 + suit`, so `suit = (i mod 52) mod 4`.
///
/// Suits are NOT consensus — the circuit and every payout depend only on the
/// rank codes. They exist so the display layer can show the exact physical
/// card the fair shuffle drew instead of inventing a suit pattern.
#[inline]
pub fn shoe_layout_suit(i: usize) -> u8 {
    ((i % 52) % 4) as u8
}

/// Deal the shoe with a deterministic `MAX_CARDS`-swap partial Fisher–Yates
/// over the fixed 416-card layout, returning the dealt rank codes.
///
/// `swap_random[k]` is the low 32 bits of the caller-computed RNG for step `k`
/// (`low32(Poseidon2(mix ‖ k))`, with `mix = Poseidon2(server_seed ‖
/// user_secret_random)`); the swap partner is `j = k + (swap_random[k] mod
/// (SHOE_SIZE − k))`. Pure — no hashing lives here.
pub fn deal_shoe(swap_random: &[u64; MAX_CARDS]) -> [u8; MAX_CARDS] {
    deal_shoe_with_suits(swap_random).0
}

/// Like [`deal_shoe`], but also returns the cosmetic suit codes of the same
/// physical cards the shuffle picked (`suits[k]` belongs to `ranks[k]`).
///
/// The permutation is identical to [`deal_shoe`] — the same swaps applied to
/// the full 416 layout positions instead of the pre-mapped rank codes — so the
/// rank output is byte-for-byte the consensus shoe, and the suits are the ones
/// physically sitting at the drawn positions (display-only, see
/// [`shoe_layout_suit`]).
pub fn deal_shoe_with_suits(
    swap_random: &[u64; MAX_CARDS],
) -> ([u8; MAX_CARDS], [u8; MAX_CARDS]) {
    let mut arr: [u16; SHOE_SIZE] = core::array::from_fn(|i| i as u16);
    for k in 0..MAX_CARDS {
        let m = (SHOE_SIZE - k) as u64;
        let j = k + (swap_random[k] % m) as usize;
        arr.swap(k, j);
    }
    let mut ranks = [0u8; MAX_CARDS];
    let mut suits = [0u8; MAX_CARDS];
    for k in 0..MAX_CARDS {
        ranks[k] = shoe_layout_code(arr[k] as usize);
        suits[k] = shoe_layout_suit(arr[k] as usize);
    }
    (ranks, suits)
}

/// Pack an action sequence into the two base-8 halves the rollup records as
/// `prediction_lo` / `prediction_hi`: `actions[0..10] → lo`, `actions[10..20] →
/// hi`, each folded as `Σ action[i] · 8^i`.
pub fn pack_actions(actions: &[u8]) -> (u32, u32) {
    let half = MAX_ACTIONS / 2;
    let mut lo = 0u64;
    let mut base = 1u64;
    for i in 0..half {
        lo += actions.get(i).copied().unwrap_or(0) as u64 * base;
        base *= ACTION_PACK_BASE;
    }
    let mut hi = 0u64;
    let mut base = 1u64;
    for i in half..MAX_ACTIONS {
        hi += actions.get(i).copied().unwrap_or(0) as u64 * base;
        base *= ACTION_PACK_BASE;
    }
    (lo as u32, hi as u32)
}

/// Unpack the base-8 action halves into the zero-terminated action prefix the
/// engine replays. Code `0` means "no action", so the real round is the prefix
/// before the first zero — the inverse of [`pack_actions`].
pub fn unpack_actions(prediction_lo: u32, prediction_hi: u32) -> Vec<u8> {
    let half = MAX_ACTIONS / 2;
    let mut out = Vec::with_capacity(MAX_ACTIONS);
    let mut lo = prediction_lo as u64;
    for _ in 0..half {
        out.push((lo % ACTION_PACK_BASE) as u8);
        lo /= ACTION_PACK_BASE;
    }
    let mut hi = prediction_hi as u64;
    for _ in 0..half {
        out.push((hi % ACTION_PACK_BASE) as u8);
        hi /= ACTION_PACK_BASE;
    }
    let count = out.iter().take_while(|&&a| a != 0).count();
    out.truncate(count);
    out
}

// ── Internal hand-kind codes (mirror the JS `TYPE_ENUM`) ─────────────────────
const KIND_BET: u8 = 0;
const KIND_HIT: u8 = 1;
const KIND_STAND: u8 = 2;
const KIND_DOUBLE: u8 = 3;
const KIND_SPLIT: u8 = 4;
const KIND_INSURANCE: u8 = 5;

// Win coefficients (JS `coefficients`): win = 2.0, blackjack = 2.5.
// Expressed as exact rational num/den so payouts stay in integer arithmetic.
const WIN_NUM: u128 = 2;
const WIN_DEN: u128 = 1;
const BJ_NUM: u128 = 5;
const BJ_DEN: u128 = 2;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Play,
    Lose,
    Cashout,
}

#[derive(Clone)]
struct Hand {
    cards: Vec<u8>,
    /// Shoe draw index of each card in `cards` (parallel array). Display-only
    /// bookkeeping so consumers can restore the cosmetic suit of the exact
    /// physical card — never consumed by the payout logic.
    idxs: Vec<u8>,
    amount: u64,
    win_amount: u64,
    kind: u8,
    points: u32,
}

struct State {
    first: Hand,
    second: Option<Hand>,
    dealer_cards: Vec<u8>,
    /// Shoe draw index of each dealer card (parallel to `dealer_cards`).
    dealer_idxs: Vec<u8>,
    insurance_amount: u64,
    amount: u64,
    win_amount: u64,
    /// Running win total before the final MAX_WIN cap.
    win_uncapped: u64,
    status: Status,
    kind: u8,
}

/// Detailed outcome of a replayed round.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlackjackResult {
    pub payout: GamePayout,
    /// Sum of every stake placed this round (base + double + split + insurance),
    /// in atomic units. This is the `bet` that the rollup slot records.
    pub total_staked: u64,
    /// Total win before the MAX_WIN cap (the ZK divide-and-cap `raw_win`).
    pub win_uncapped: u64,
    /// Dealer's final hand total (used as `roll_number`).
    pub dealer_points: u32,
    /// First hand's final points (settle-time), for the ZK `dp < fp` hint.
    pub first_points: u32,
    /// Second hand's final points, or 0 when the round stayed single-hand.
    pub second_points: u32,
    /// 1 if the round stayed single-hand, 2 if a split created a second hand.
    pub num_hands: u8,
    /// First hand's terminal `kind` (`0`=bet, `1`=hit, `2`=stand, `3`=double,
    /// `4`=split, `5`=insurance). Display/verification only — NOT consumed by the
    /// circuit. Any *closed* hand (explicit stand, double, bust, a natural/drawn
    /// 21, or a 10-card charlie) collapses to `2` (stand), so a consumer can treat
    /// `first_kind == 2` as "the first hand can no longer act". Mirrors the JS
    /// `firstHand.type` the frontend reads to highlight the active hand.
    pub first_kind: u8,
    /// Second hand's terminal `kind` (same encoding as [`Self::first_kind`]), or
    /// `0` (bet) when the round stayed single-hand (no split). Mirrors the JS
    /// `secondHand.type`.
    pub second_kind: u8,
    /// First hand's staked amount in atomic units — the base bet, or `2×` after a
    /// double. Display only; the authoritative round total is [`Self::total_staked`].
    pub first_amount: u64,
    /// Second hand's staked amount in atomic units, or `0` when the round stayed
    /// single-hand (no split).
    pub second_amount: u64,
    /// First hand's win in atomic units (stake returned + profit), BEFORE the
    /// round-level `MAX_WIN` cap. Display/history only; the capped, authoritative
    /// payout is the round total [`Self::win_amount`] (`payout.win_amount`).
    pub first_win: u64,
    /// Second hand's win in atomic units (pre-cap), or `0` when single-hand.
    pub second_win: u64,
    /// Whether a positive insurance stake was standing at settle.
    pub insurance_taken: bool,
    /// Whether the action sequence brought the round to a settled state.
    pub is_finished: bool,
    /// First hand's final cards (rank codes 0..=12). The exact cards dealt to the
    /// player's first hand — display/verification only, not consumed by the circuit.
    pub first_cards: Vec<u8>,
    /// Second hand's final cards (rank codes), or empty when the round stayed
    /// single-hand (no split).
    pub second_cards: Vec<u8>,
    /// Dealer's final cards (rank codes), including the revealed hole card and any
    /// draws.
    pub dealer_cards: Vec<u8>,
    /// Shoe draw index (0..30) of each card in [`Self::first_cards`] — which
    /// position of the dealt shoe the card came from. Display-only: paired with
    /// the suits from [`deal_shoe_with_suits`] it restores the cosmetic suit of
    /// the exact physical card the fair shuffle drew. NOT consumed by the
    /// circuit or any payout.
    pub first_card_indices: Vec<u8>,
    /// Shoe draw indices of [`Self::second_cards`], or empty when single-hand.
    pub second_card_indices: Vec<u8>,
    /// Shoe draw indices of [`Self::dealer_cards`].
    pub dealer_card_indices: Vec<u8>,
}

/// Blackjack points for a hand, with soft-ace handling identical to the JS
/// engine: every ace counts as 1, then each ace is upgraded by +10 while the
/// total stays ≤ 21.
pub fn get_hand_points(cards: &[u8]) -> u32 {
    let mut ace_count = 0u32;
    let mut points = 0u32;
    for &c in cards {
        if c == 0 {
            ace_count += 1;
            points += 1;
        } else {
            points += card_value(c);
        }
    }
    for _ in 0..ace_count {
        if points + 10 <= POINTS_LIMIT {
            points += 10;
        } else {
            break;
        }
    }
    points
}

/// Hard value of a non-ace card (ace is handled separately in `get_hand_points`).
fn card_value(code: u8) -> u32 {
    match code {
        0 => 1,             // ace hard value
        1..=8 => code as u32 + 1, // 2..9
        _ => 10,            // 10 / J / Q / K
    }
}

#[inline]
fn mul_coeff(amount: u64, num: u128, den: u128) -> u64 {
    (amount as u128 * num / den) as u64
}

fn subtype_of(action: u8) -> u8 {
    match action {
        ACTION_HIT => KIND_HIT,
        ACTION_STAND => KIND_STAND,
        ACTION_DOUBLE => KIND_DOUBLE,
        ACTION_SPLIT => KIND_SPLIT,
        ACTION_INSURANCE_ACCEPT | ACTION_INSURANCE_DECLINE => KIND_INSURANCE,
        _ => panic!("unknown blackjack action code {action}"),
    }
}

impl State {
    fn deal(cards: &[u8], bet: u64) -> Self {
        assert!(cards.len() >= 3, "deck needs at least 3 cards for a deal");
        State {
            first: Hand {
                cards: vec![cards[0], cards[1]],
                idxs: vec![0, 1],
                amount: bet,
                win_amount: 0,
                kind: KIND_BET,
                points: 0,
            },
            second: None,
            dealer_cards: vec![cards[2]],
            dealer_idxs: vec![2],
            insurance_amount: 0,
            amount: bet,
            win_amount: 0,
            win_uncapped: 0,
            status: Status::Play,
            kind: KIND_BET,
        }
    }

    fn finished(&self) -> bool {
        matches!(self.status, Status::Cashout | Status::Lose)
    }

    fn active_is_first(&self) -> bool {
        self.first.kind != KIND_STAND
    }

    fn second_len(&self) -> usize {
        self.second.as_ref().map(|h| h.cards.len()).unwrap_or(0)
    }

    /// Reproduce `getNextCard`: index of the next card to draw from the shoe.
    fn next_index(&self) -> usize {
        let dealer_len = self.dealer_cards.len().max(2);
        self.first.cards.len() + self.second_len() + dealer_len
    }

    fn card_at(cards: &[u8], idx: usize) -> u8 {
        assert!(idx < cards.len(), "deck exhausted at index {idx}");
        cards[idx]
    }

    // ── slowBet: applyRoundAction (book-keep extra stakes) ───────────────
    fn apply_action(&mut self, action: u8) {
        let active_first = self.active_is_first();
        match action {
            ACTION_DOUBLE => {
                let extra = self.active_amount(active_first);
                self.amount += extra;
                self.add_active_amount(active_first, extra);
            }
            ACTION_SPLIT => {
                let extra = self.active_amount(active_first);
                self.amount += extra;
                self.second = Some(Hand {
                    cards: Vec::new(),
                    idxs: Vec::new(),
                    amount: extra,
                    win_amount: 0,
                    kind: KIND_BET,
                    points: 0,
                });
            }
            ACTION_INSURANCE_ACCEPT => {
                let extra = self.active_amount(active_first) / 2;
                self.insurance_amount = extra;
                if extra > 0 {
                    self.amount += extra;
                }
            }
            ACTION_INSURANCE_DECLINE => {
                self.insurance_amount = 0;
            }
            _ => {}
        }
    }

    fn active_amount(&self, active_first: bool) -> u64 {
        if active_first {
            self.first.amount
        } else {
            self.second.as_ref().expect("no active hand").amount
        }
    }

    fn add_active_amount(&mut self, active_first: bool, extra: u64) {
        if active_first {
            self.first.amount += extra;
        } else {
            self.second.as_mut().expect("no active hand").amount += extra;
        }
    }

    // ── slowBet: fillRound (set active hand kind + betDoc.type) ──────────
    fn fill_round(&mut self, action: u8) {
        let active_first = self.active_is_first();
        let sub = subtype_of(action);
        if active_first {
            self.first.kind = sub;
        } else if let Some(h) = self.second.as_mut() {
            h.kind = sub;
        }
        self.kind = KIND_HIT; // betDoc.type = 1 (any action)
    }

    fn calculate_round(&mut self, cards: &[u8]) {
        match self.kind {
            0 => self.calc_deal(),
            1 => self.calc_action(cards),
            2 => self.calc_final(),
            _ => {}
        }
    }

    // ── calculateRound, betDoc.type === 0 (deal) ─────────────────────────
    fn calc_deal(&mut self) {
        self.first.points = get_hand_points(&self.first.cards);
        if self.first.points == POINTS_LIMIT {
            self.first.kind = KIND_STAND;
            self.status = Status::Cashout;
            self.kind = KIND_STAND; // betDoc.type = 2
        }
    }

    // ── calculateRound, betDoc.type === 1 (an action was applied) ────────
    fn calc_action(&mut self, cards: &[u8]) {
        let active_first = self.active_is_first();

        let (active_exists, active_kind) = if active_first {
            (true, self.first.kind)
        } else {
            match &self.second {
                Some(h) => (true, h.kind),
                None => (false, 0),
            }
        };

        // hand === undefined || hand.type === stand → cash out, stop.
        if !active_exists || active_kind == KIND_STAND {
            self.kind = KIND_STAND; // betDoc.type = 2
            self.status = Status::Cashout;
            return;
        }

        match active_kind {
            KIND_HIT => {
                let idx = self.next_index();
                let c = Self::card_at(cards, idx);
                self.push_active(active_first, c, idx as u8);
                if self.active_cards_len(active_first) >= 10 {
                    self.set_active_kind(active_first, KIND_STAND);
                }
            }
            KIND_DOUBLE => {
                let idx = self.next_index();
                let c = Self::card_at(cards, idx);
                self.push_active(active_first, c, idx as u8);
                self.set_active_kind(active_first, KIND_STAND);
            }
            KIND_SPLIT => {
                // Split only ever acts on the first hand (guaranteed by the
                // legacy validator: no existing second hand, first hand is a
                // fresh pair). `applyRoundAction` already created the empty
                // second hand.
                debug_assert!(active_first, "split must act on the first hand");
                let is_two_aces = self.first.cards[0] == 0;
                let cards_count = self.first.cards.len() + self.second_len() + 2;
                let second_card = self.first.cards.remove(0); // Array.shift()
                let second_idx = self.first.idxs.remove(0);
                let c0 = Self::card_at(cards, cards_count);
                let c1 = Self::card_at(cards, cards_count + 1);
                self.first.cards.push(c0);
                self.first.idxs.push(cards_count as u8);
                let new_kind = if is_two_aces { KIND_STAND } else { KIND_BET };
                self.first.kind = new_kind;
                let split_amount = self.first.amount;
                self.second = Some(Hand {
                    cards: vec![second_card, c1],
                    idxs: vec![second_idx, (cards_count + 1) as u8],
                    amount: split_amount,
                    win_amount: 0,
                    kind: new_kind,
                    points: 0,
                });
            }
            KIND_INSURANCE => {
                let hole = Self::card_at(cards, 3);
                let mut peek = self.dealer_cards.clone();
                peek.push(hole);
                if self.insurance_amount > 0 && get_hand_points(&peek) == POINTS_LIMIT {
                    self.first.kind = KIND_STAND;
                    self.status = Status::Cashout;
                    self.kind = KIND_STAND;
                    return;
                }
                self.set_active_kind(active_first, KIND_BET);
            }
            _ => {}
        }

        // hand.points = getHandPoints(hand.cards); if >= 21 → stand.
        let ap = if active_first {
            get_hand_points(&self.first.cards)
        } else {
            get_hand_points(&self.second.as_ref().unwrap().cards)
        };
        if active_first {
            self.first.points = ap;
            if ap >= POINTS_LIMIT {
                self.first.kind = KIND_STAND;
            }
        } else {
            let h = self.second.as_mut().unwrap();
            h.points = ap;
            if ap >= POINTS_LIMIT {
                h.kind = KIND_STAND;
            }
        }

        // secondHand && (secondHand.points = ...); if >= 21 → stand.
        if let Some(h) = self.second.as_mut() {
            let sp = get_hand_points(&h.cards);
            h.points = sp;
            if sp >= POINTS_LIMIT {
                h.kind = KIND_STAND;
            }
        }

        let second_stand = self
            .second
            .as_ref()
            .map(|h| h.kind == KIND_STAND)
            .unwrap_or(true);
        if self.first.kind == KIND_STAND && second_stand {
            self.status = Status::Cashout;
        }

        let second_bust = self
            .second
            .as_ref()
            .map(|h| h.points > POINTS_LIMIT)
            .unwrap_or(true);
        if self.first.points > POINTS_LIMIT && second_bust {
            self.status = Status::Lose;

            if self.insurance_amount > 0 {
                let hole = Self::card_at(cards, 3);
                self.dealer_cards.push(hole);
                self.dealer_idxs.push(3);
                let dp = get_hand_points(&self.dealer_cards);
                if dp == POINTS_LIMIT {
                    // Insurance pays 2:1: winnings (2x stake) + the stake itself back = 3x.
                    self.win_amount += self.insurance_amount * 3;
                }
            }
        }
    }

    // ── calculateRound, betDoc.type === 2 (settle) ───────────────────────
    fn calc_final(&mut self) {
        // Own the second hand locally so the whole settle stays borrow-clean.
        let mut second = self.second.take();

        let dealer_points = get_hand_points(&self.dealer_cards);
        let fh_points = get_hand_points(&self.first.cards);
        let sh_points = match &second {
            Some(h) => get_hand_points(&h.cards),
            None => 0,
        };
        self.first.points = fh_points;
        if let Some(h) = second.as_mut() {
            h.points = sh_points;
        }

        let dealer_len = self.dealer_cards.len();
        let first_len = self.first.cards.len();

        if dealer_len == 2 && dealer_points == POINTS_LIMIT {
            // Dealer natural blackjack.
            if sh_points == 0 && fh_points == POINTS_LIMIT && first_len == 2 {
                self.first.win_amount = self.first.amount;
                self.win_amount += self.first.amount;
            }
            if self.insurance_amount > 0 {
                // Insurance pays 2:1: winnings (2x stake) + the stake itself back = 3x.
                self.win_amount += self.insurance_amount * 3;
            }
        } else {
            // Push on the first hand (equal totals).
            if dealer_points == fh_points && dealer_points <= POINTS_LIMIT {
                let bj_push = fh_points == POINTS_LIMIT
                    && second.is_none()
                    && first_len == 2
                    && dealer_len > 2;
                self.first.win_amount = if bj_push {
                    mul_coeff(self.first.amount, BJ_NUM, BJ_DEN)
                } else {
                    self.first.amount
                };
                self.win_amount += self.first.win_amount;
            }
            // Push on the second hand.
            if let Some(h) = second.as_mut() {
                if dealer_points == sh_points && dealer_points <= POINTS_LIMIT {
                    h.win_amount = h.amount;
                    self.win_amount += h.amount;
                }
            }
        }

        // First hand beats the dealer (dealer lower or busts).
        if fh_points <= POINTS_LIMIT && (dealer_points < fh_points || dealer_points > POINTS_LIMIT) {
            let is_bj = fh_points == POINTS_LIMIT && second.is_none() && first_len == 2;
            self.first.win_amount = if is_bj {
                mul_coeff(self.first.amount, BJ_NUM, BJ_DEN)
            } else {
                mul_coeff(self.first.amount, WIN_NUM, WIN_DEN)
            };
            self.win_amount += self.first.win_amount;
        }

        // 10-card charlie upgrade for a pushed first hand.
        if self.first.win_amount == self.first.amount && first_len >= 10 && fh_points <= POINTS_LIMIT {
            self.first.win_amount = mul_coeff(self.first.amount, WIN_NUM, WIN_DEN);
            self.win_amount += self.first.win_amount - self.first.amount;
        }

        if let Some(h) = second.as_mut() {
            let sa = h.amount;
            let s_len = h.cards.len();
            // Second hand beats the dealer.
            if sh_points > 0
                && sh_points <= POINTS_LIMIT
                && (dealer_points < sh_points || dealer_points > POINTS_LIMIT)
            {
                h.win_amount = mul_coeff(sa, WIN_NUM, WIN_DEN);
                self.win_amount += h.win_amount;
            }
            // 10-card charlie upgrade for a pushed second hand.
            if h.win_amount == sa && s_len >= 10 && sh_points <= POINTS_LIMIT {
                h.win_amount = mul_coeff(sa, WIN_NUM, WIN_DEN);
                self.win_amount += h.win_amount - sa;
            }
        }

        self.win_uncapped = self.win_amount;
        self.win_amount = self.win_amount.min(MAX_WIN);
        self.second = second;
    }

    fn close_round(&mut self, cards: &[u8]) {
        self.kind = KIND_STAND; // betDoc.type = 2
        if self.status == Status::Cashout {
            let hole = Self::card_at(cards, 3);
            self.dealer_cards.push(hole);
            self.dealer_idxs.push(3);
            let draw = self.second.is_some()
                || self.first.cards.len() > 2
                || self.first.points != POINTS_LIMIT;
            if draw {
                while get_hand_points(&self.dealer_cards) < DEALER_POINTS_LIMIT {
                    let idx = self.next_index();
                    let c = Self::card_at(cards, idx);
                    self.dealer_cards.push(c);
                    self.dealer_idxs.push(idx as u8);
                }
            }
        }
    }

    fn push_active(&mut self, active_first: bool, card: u8, idx: u8) {
        if active_first {
            self.first.cards.push(card);
            self.first.idxs.push(idx);
        } else {
            let h = self.second.as_mut().expect("no active hand");
            h.cards.push(card);
            h.idxs.push(idx);
        }
    }

    fn active_cards_len(&self, active_first: bool) -> usize {
        if active_first {
            self.first.cards.len()
        } else {
            self.second.as_ref().expect("no active hand").cards.len()
        }
    }

    fn set_active_kind(&mut self, active_first: bool, kind: u8) {
        if active_first {
            self.first.kind = kind;
        } else {
            self.second.as_mut().expect("no active hand").kind = kind;
        }
    }
}

/// Replay a full blackjack round and return the rich outcome.
///
/// - `cards_values`: the already-dealt shoe prefix as rank codes (see module
///   docs). Must be long enough for every card the round consumes.
/// - `actions_packed`: ordered player actions (see `ACTION_*`).
/// - `bet_atomic`: the base stake in atomic units.
pub fn replay_full(cards_values: &[u8], actions_packed: &[u8], bet_atomic: u64) -> BlackjackResult {
    assert!(
        cards_values.len() <= MAX_CARDS,
        "at most {MAX_CARDS} cards per round, got {}",
        cards_values.len()
    );

    let mut st = State::deal(cards_values, bet_atomic);
    st.calculate_round(cards_values); // type 0 (deal)

    if !st.finished() {
        for &action in actions_packed {
            st.apply_action(action);
            st.fill_round(action);
            st.calculate_round(cards_values); // type 1
            if st.finished() {
                break;
            }
        }
    }

    if st.finished() {
        st.close_round(cards_values);
        st.calculate_round(cards_values); // type 2 (settle)
    }

    let dealer_points = get_hand_points(&st.dealer_cards);
    let win_amount = st.win_amount;
    let total_staked = st.amount;
    let multiplier = if total_staked > 0 {
        (win_amount as u128 * crate::shared::PAYOUT_DIVISOR as u128 / total_staked as u128) as u64
    } else {
        0
    };
    let first_points = get_hand_points(&st.first.cards);
    let second_points = st
        .second
        .as_ref()
        .map(|h| get_hand_points(&h.cards))
        .unwrap_or(0);
    let first_cards = st.first.cards.clone();
    let second_cards = st
        .second
        .as_ref()
        .map(|h| h.cards.clone())
        .unwrap_or_default();
    let dealer_cards = st.dealer_cards.clone();
    let first_card_indices = st.first.idxs.clone();
    let second_card_indices = st
        .second
        .as_ref()
        .map(|h| h.idxs.clone())
        .unwrap_or_default();
    let dealer_card_indices = st.dealer_idxs.clone();
    // Terminal per-hand kinds for the display layer (frontend active-hand
    // highlight). A closed hand always ends as KIND_STAND; the second hand's
    // kind is meaningless without a split, so report bet (0) then.
    let first_kind = st.first.kind;
    let second_kind = st.second.as_ref().map(|h| h.kind).unwrap_or(KIND_BET);
    // Per-hand staked/win breakdown for the display layer (win is pre-cap; the
    // capped round total lives in `win_amount`). Absent second hand → zeros.
    let first_amount = st.first.amount;
    let first_win = st.first.win_amount;
    let (second_amount, second_win) = st
        .second
        .as_ref()
        .map(|h| (h.amount, h.win_amount))
        .unwrap_or((0, 0));

    BlackjackResult {
        payout: GamePayout {
            win_amount,
            roll_number: dealer_points,
            is_win: win_amount > 0,
            multiplier,
        },
        total_staked,
        win_uncapped: st.win_uncapped,
        dealer_points,
        first_points,
        second_points,
        num_hands: 1 + st.second.is_some() as u8,
        first_kind,
        second_kind,
        first_amount,
        second_amount,
        first_win,
        second_win,
        insurance_taken: st.insurance_amount > 0,
        is_finished: st.finished(),
        first_cards,
        second_cards,
        dealer_cards,
        first_card_indices,
        second_card_indices,
        dealer_card_indices,
    }
}

/// Replay a full blackjack round and return only the payout.
///
/// See [`replay_full`] for the richer result (total staked, dealer points).
pub fn replay(cards_values: &[u8], actions_packed: &[u8], bet_atomic: u64) -> GamePayout {
    replay_full(cards_values, actions_packed, bet_atomic).payout
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_unpack_round_trips() {
        let seq = [
            ACTION_HIT,
            ACTION_DOUBLE,
            ACTION_STAND,
            ACTION_SPLIT,
            ACTION_INSURANCE_ACCEPT,
        ];
        let (lo, hi) = pack_actions(&seq);
        assert_eq!(unpack_actions(lo, hi), seq.to_vec());

        // A lone action packs into `lo`'s lowest base-8 digit; the prefix drops
        // the zero padding.
        let (lo, hi) = pack_actions(&[ACTION_STAND]);
        assert_eq!(unpack_actions(lo, hi), vec![ACTION_STAND]);
        assert_eq!(unpack_actions(0, 0), Vec::<u8>::new());
    }

    #[test]
    fn deal_shoe_is_a_permutation_of_the_layout() {
        // Zero randomness ⇒ every swap is a no-op ⇒ the first 30 layout codes.
        let no_swap = [0u64; MAX_CARDS];
        let drawn = deal_shoe(&no_swap);
        let expected: [u8; MAX_CARDS] = core::array::from_fn(shoe_layout_code);
        assert_eq!(drawn, expected);

        // A fixed non-trivial swap vector is deterministic and stays in range.
        let swaps: [u64; MAX_CARDS] = core::array::from_fn(|k| (k as u64) * 2_654_435_761);
        let a = deal_shoe(&swaps);
        assert_eq!(a, deal_shoe(&swaps), "deterministic");
        assert!(a.iter().all(|&c| (c as usize) < 13), "rank codes 0..12");
    }

    #[test]
    fn deal_shoe_with_suits_matches_deal_shoe() {
        let swaps: [u64; MAX_CARDS] = core::array::from_fn(|k| (k as u64) * 2_654_435_761);
        let (ranks, suits) = deal_shoe_with_suits(&swaps);
        assert_eq!(ranks, deal_shoe(&swaps), "rank output must stay consensus-identical");
        assert!(suits.iter().all(|&s| s < 4), "suit codes 0..3");

        // Zero randomness ⇒ unshuffled layout: rank (i%52)/4, suit (i%52)%4.
        let (r0, s0) = deal_shoe_with_suits(&[0u64; MAX_CARDS]);
        for i in 0..MAX_CARDS {
            assert_eq!(r0[i], shoe_layout_code(i));
            assert_eq!(s0[i], shoe_layout_suit(i));
        }
    }

    #[test]
    fn replay_reports_shoe_indices_of_each_card() {
        // Player [10,10]=20 stands; dealer up 8, hole 9 → 17, no draws.
        let deck = {
            let mut d = [0u8; MAX_CARDS];
            d[..4].copy_from_slice(&[9, 9, 7, 8]);
            d
        };
        let r = replay_full(&deck, &[ACTION_STAND], 1_000_000);
        assert_eq!(r.first_card_indices, vec![0, 1]);
        assert_eq!(r.dealer_card_indices, vec![2, 3]);

        // Split: [5,5] vs dealer 6/10. cards_count = 4, so the first hand keeps
        // shoe card 1 and draws 4; the second hand takes shoe cards 0 and 5.
        let deck = {
            let mut d = [0u8; MAX_CARDS];
            //             p0 p1 up hole s1 s2 dealer draws...
            d[..8].copy_from_slice(&[4, 4, 5, 9, 8, 7, 9, 9]);
            d
        };
        let r = replay_full(&deck, &[ACTION_SPLIT, ACTION_STAND, ACTION_STAND], 1_000_000);
        assert_eq!(r.num_hands, 2);
        assert_eq!(r.first_cards, vec![deck[1], deck[4]]);
        assert_eq!(r.first_card_indices, vec![1, 4]);
        assert_eq!(r.second_cards, vec![deck[0], deck[5]]);
        assert_eq!(r.second_card_indices, vec![0, 5]);
        assert_eq!(r.dealer_cards[..2], [deck[2], deck[3]]);
        assert_eq!(r.dealer_card_indices[..2], [2, 3]);
        // Any dealer draws continue from the shared frontier.
        for (n, &idx) in r.dealer_card_indices[2..].iter().enumerate() {
            assert_eq!(idx as usize, 6 + n);
        }
    }
}
