use rolly_game_core::plinko::*;
use rolly_game_core::shared::*;

const TARGET_RTP: f64 = 99.0;

fn binom(n: u32, k: u32) -> u128 {
    if k > n { return 0; }
    let mut result = 1u128;
    for i in 0..k {
        result = result * (n - i) as u128 / (i + 1) as u128;
    }
    result
}

fn analytical_rtp(rows: u32, table: &[u64]) -> f64 {
    let total = 1u128 << rows;
    let mut weighted = 0u128;
    for k in 0..=rows {
        weighted += binom(rows, k) * table[k as usize] as u128;
    }
    weighted as f64 / (total as f64 * PAYOUT_DIVISOR as f64) * 100.0
}

/// Scale multipliers so the table hits exactly TARGET_RTP.
/// Each multiplier is scaled by (target / current_rtp), then floored to u64.
/// After flooring, any remaining deficit is added to the middle bucket(s)
/// to hit the target sum exactly.
fn rescale_table(rows: u32, old_table: &[u64]) -> Vec<u64> {
    let total = 1u128 << rows;
    let target_weighted_sum = (TARGET_RTP / 100.0 * total as f64 * PAYOUT_DIVISOR as f64) as u128;

    let current_rtp = analytical_rtp(rows, old_table);
    let scale = TARGET_RTP / current_rtp;

    let mut new_table: Vec<u64> = old_table.iter()
        .map(|&m| (m as f64 * scale).floor() as u64)
        .collect();

    let mut current_sum = 0u128;
    for k in 0..=rows {
        current_sum += binom(rows, k) * new_table[k as usize] as u128;
    }

    let deficit = target_weighted_sum as i128 - current_sum as i128;

    if deficit > 0 {
        let mid = rows as usize / 2;
        let mid_binom = binom(rows, mid as u32) as i128;
        let add = (deficit + mid_binom - 1) / mid_binom;
        new_table[mid] += add as u64;
    }

    new_table
}

#[test]
fn plinko_recalc_99_rtp() {
    println!();
    println!("=== PLINKO TABLES RECALCULATED FOR {:.1}% RTP ===", TARGET_RTP);
    println!();

    for (sector, rows, is_extreme, old_table) in all_multiplier_rows() {
        let label = format!("s{}r{}{}", sector, rows, if is_extreme { "X" } else { "" });
        let old_rtp = analytical_rtp(rows, old_table);
        let new_table = rescale_table(rows, old_table);
        let new_rtp = analytical_rtp(rows, &new_table);

        println!("// {label}  (was {old_rtp:.4}% -> now {new_rtp:.4}%)");

        let is_array = rows + 1;
        let name = if is_extreme {
            format!("E_S{}_R{}", sector, rows)
        } else {
            format!("N_S{}_R{}", sector, rows)
        };

        let values: Vec<String> = new_table.iter().map(|v| format!("{}", v)).collect();
        println!("pub const {}: [u64; {}] = [{}];",
            name, is_array, values.join(", "));
        println!();
    }

    println!();
    println!("=== VERIFICATION ===");
    println!();
    println!("{:<12} {:>10} {:>10} {:>10}", "Config", "Old RTP", "New RTP", "Edge");
    println!("{}", "-".repeat(46));

    for (sector, rows, is_extreme, old_table) in all_multiplier_rows() {
        let label = format!("s{}r{}{}", sector, rows, if is_extreme { "X" } else { "" });
        let old_rtp = analytical_rtp(rows, old_table);
        let new_table = rescale_table(rows, old_table);
        let new_rtp = analytical_rtp(rows, &new_table);

        println!("{:<12} {:>10.4}% {:>10.4}% {:>10.4}%", label, old_rtp, new_rtp, 100.0 - new_rtp);
    }
    println!();
}
