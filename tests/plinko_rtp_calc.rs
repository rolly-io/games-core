use rolly_game_core::plinko::*;
use rolly_game_core::shared::*;

fn binom(n: u32, k: u32) -> u128 {
    if k > n { return 0; }
    let mut result = 1u128;
    for i in 0..k {
        result = result * (n - i) as u128 / (i + 1) as u128;
    }
    result
}

/// Analytical RTP for one Plinko config.
/// RTP = sum( C(rows, k) * multiplier[k] ) / (2^rows * PAYOUT_DIVISOR)
fn analytical_rtp(rows: u32, table: &[u64]) -> f64 {
    let total_outcomes = 1u128 << rows; // 2^rows
    let mut weighted_sum = 0u128;

    for k in 0..=rows {
        let prob_numerator = binom(rows, k); // C(rows, k)
        weighted_sum += prob_numerator * table[k as usize] as u128;
    }

    // RTP = weighted_sum / (total_outcomes * PAYOUT_DIVISOR)
    weighted_sum as f64 / (total_outcomes as f64 * PAYOUT_DIVISOR as f64) * 100.0
}

#[test]
fn plinko_analytical_rtp_all_configs() {
    println!();
    println!("=== PLINKO ANALYTICAL RTP (exact, no simulation) ===");
    println!();
    println!("{:<12} {:>8} {:>10} {:>10}", "Config", "Rows", "RTP %", "Edge %");
    println!("{}", "-".repeat(44));

    let mut all_rtp = Vec::new();

    for (sector, rows, is_extreme, table) in all_multiplier_rows() {
        let label = format!("s{}r{}{}", sector, rows, if is_extreme { "X" } else { "" });
        let rtp = analytical_rtp(rows, table);
        let edge = 100.0 - rtp;
        all_rtp.push((label.clone(), rtp));

        println!("{:<12} {:>8} {:>10.6} {:>10.6}", label, rows, rtp, edge);
    }

    println!();
    println!("=== SUMMARY ===");
    println!();

    let normal_s0: Vec<f64> = all_rtp.iter()
        .filter(|(l, _)| l.starts_with("s0r") && !l.ends_with("X"))
        .map(|(_, r)| *r).collect();
    let normal_s1: Vec<f64> = all_rtp.iter()
        .filter(|(l, _)| l.starts_with("s1r") && !l.ends_with("X"))
        .map(|(_, r)| *r).collect();
    let normal_s2: Vec<f64> = all_rtp.iter()
        .filter(|(l, _)| l.starts_with("s2r") && !l.ends_with("X"))
        .map(|(_, r)| *r).collect();
    let extreme: Vec<f64> = all_rtp.iter()
        .filter(|(l, _)| l.ends_with("X"))
        .map(|(_, r)| *r).collect();

    let avg = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    let min = |v: &[f64]| v.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = |v: &[f64]| v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    println!("Sector 0 (Low):    avg={:.4}%  min={:.4}%  max={:.4}%", avg(&normal_s0), min(&normal_s0), max(&normal_s0));
    println!("Sector 1 (Medium): avg={:.4}%  min={:.4}%  max={:.4}%", avg(&normal_s1), min(&normal_s1), max(&normal_s1));
    println!("Sector 2 (High):   avg={:.4}%  min={:.4}%  max={:.4}%", avg(&normal_s2), min(&normal_s2), max(&normal_s2));
    println!("Extreme:           avg={:.4}%  min={:.4}%  max={:.4}%", avg(&extreme), min(&extreme), max(&extreme));
    println!();

    // Show per-bucket breakdown for extreme configs
    for (sector, rows, is_extreme, table) in all_multiplier_rows() {
        if !is_extreme { continue; }
        let label = format!("s{}r{}X", sector, rows);
        let total = 1u128 << rows;

        println!("--- {} bucket breakdown ---", label);
        println!("{:>7} {:>12} {:>14} {:>12} {:>10}", "Bucket", "Multiplier", "P(bucket)", "Contrib", "Contrib%");

        let mut total_contrib = 0.0f64;
        for k in 0..=rows {
            let prob_num = binom(rows, k);
            let prob = prob_num as f64 / total as f64;
            let multi_real = table[k as usize] as f64 / PAYOUT_DIVISOR as f64;
            let contrib = prob * multi_real;
            total_contrib += contrib;

            if table[k as usize] > 0 {
                println!("{:>7} {:>12} {:>14.10} {:>12.8} {:>9.4}%",
                    k, format!("{:.4}x", multi_real), prob, contrib, contrib * 100.0);
            }
        }
        println!("{:>7} {:>12} {:>14} {:>12.8} {:>9.4}%", "TOTAL", "", "", total_contrib, total_contrib * 100.0);
        println!();
    }
}
