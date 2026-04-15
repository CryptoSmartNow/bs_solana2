use anchor_lang::prelude::*;

pub fn calculate_interest_with_bts(
    principal: u64,
    time_interval: i64,
    vault_state: u64,
    total_value_locked: u64,
) -> Result<u64> {
    if time_interval <= 0 {
        return Ok(0);
    }

    let principal = principal as u128;
    let time_interval = time_interval as u128;
    let vault_state = vault_state as u128;
    let total_value_locked = total_value_locked as u128;

    let total_supply: u128 = 15_000_000;
    let max_supply: u128 = 100_000_000;
    let year_in_seconds: u128 = 3600 * 24 * 365;
    let scale: u128 = 1_000_000_000_000_000_000; // 1e18

    // crp = ((totalSupply - vaultState).div(vaultState)).mul(100)
    let crp_fixed = if vault_state == 0 {
        scale * 100
    } else {
        ((total_supply - vault_state) * scale / vault_state) * 100 / scale
    };
    
    // crp as a percentage (roughly)
    let crp = crp_fixed;

    // bsRate = maxSupply.div(crp * totalValueLocked)
    let bs_rate_denominator = crp * total_value_locked / scale;
    let bs_rate = if bs_rate_denominator == 0 {
        scale
    } else {
        max_supply * scale / bs_rate_denominator
    };

    // yearsTaken = timeInterval.div(yearInSeconds)
    let years_taken = time_interval * scale / year_in_seconds;

    // accumulatedInterest = ((principal * bsRate * yearsTaken).div(100*divisor))
    // In EVM divisor was 1000 ether. principal is in tokens.
    // Let's assume divisor is 1000 and we stay in principal's precision.
    let divisor: u128 = 1000;
    
    // product = principal * (bs_rate/scale) * (years_taken/scale)
    let product = principal * bs_rate / scale * years_taken / scale;
    let accumulated_interest = product * scale / (100 * divisor);
    
    // The result in EVM is converted toUint() which divides by 1e18
    let result = accumulated_interest / scale;

    Ok(result as u64)
}
