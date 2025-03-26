use super::volatility::calculate_volatility;
use db_access::models::BlockHeader;
use eyre::Result;

// Calculate cap level using volatility, alpha, and k
// @param volatility: volatility of returns in BPs (e.g., 3333 for 33.33%)
// @param k: strike level in BPS (e.g., -2500 for -25%)
// @param alpha: target percentage of max returns in BPS (e.g., 5000 for 50%)

// cl = (λ - k) / (α * (1 + k))
// - λ = 2.33 x volatility: 0% <= λ < ∞%
// - k: -100.00% < k < ∞%
// - a: 0.00% < a <= 100%
pub async fn calculate_cap_level(alpha: u128, k: i128, blocks: Vec<BlockHeader>) -> Result<f64> {
    // Calculate volatility
    let volatility = calculate_volatility(blocks).await?;

    // Get percentage values for each variable
    let lambda = 2.33 * volatility;
    let alpha = (alpha as f64) / 10_000.0;
    let k = (k as f64) / 10_000.0;

    let cap_level = (lambda - k) / (alpha * (1.0 + k));

    Ok(cap_level)
}
