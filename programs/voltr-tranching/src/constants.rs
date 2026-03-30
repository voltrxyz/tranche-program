/// PDA seed for the TrancheConfig account.
/// Derivation: `[TRANCHE_CONFIG_SEED, vault.key().as_ref()]`
pub const TRANCHE_CONFIG_SEED: &[u8] = b"tranche_config";

/// PDA seed for the tranche authority signer.
/// This PDA signs all CPI calls to Voltr Vault (deposit_vault, instant_withdraw_vault)
/// and all token operations (mint_to, burn, transfer).
/// Derivation: `[TRANCHE_AUTHORITY_SEED, tranche_config.key().as_ref()]`
pub const TRANCHE_AUTHORITY_SEED: &[u8] = b"tranche_authority";

/// PDA seed for the senior tranche token mint.
/// Derivation: `[SENIOR_MINT_SEED, tranche_config.key().as_ref()]`
pub const SENIOR_MINT_SEED: &[u8] = b"senior_mint";

/// PDA seed for the junior tranche token mint.
/// Derivation: `[JUNIOR_MINT_SEED, tranche_config.key().as_ref()]`
pub const JUNIOR_MINT_SEED: &[u8] = b"junior_mint";

/// PDA seed for withdraw receipt accounts.
/// Derivation: `[WITHDRAW_RECEIPT_SEED, tranche_config.key().as_ref(), receipt_id.to_le_bytes()]`
pub const WITHDRAW_RECEIPT_SEED: &[u8] = b"withdraw_receipt";

/// PDA seed for the APR feed ring buffer.
/// Derivation: `[APR_FEED_SEED, tranche_config.key().as_ref()]`
pub const APR_FEED_SEED: &[u8] = b"apr_feed";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_seed_collisions() {
        let seeds: &[&[u8]] = &[
            TRANCHE_CONFIG_SEED,
            TRANCHE_AUTHORITY_SEED,
            SENIOR_MINT_SEED,
            JUNIOR_MINT_SEED,
            WITHDRAW_RECEIPT_SEED,
            APR_FEED_SEED,
        ];
        for i in 0..seeds.len() {
            for j in (i + 1)..seeds.len() {
                assert_ne!(
                    seeds[i], seeds[j],
                    "Seed collision between index {} and {}",
                    i, j
                );
            }
        }
    }

    #[test]
    fn test_seed_values() {
        assert_eq!(TRANCHE_CONFIG_SEED, b"tranche_config");
        assert_eq!(TRANCHE_AUTHORITY_SEED, b"tranche_authority");
        assert_eq!(SENIOR_MINT_SEED, b"senior_mint");
        assert_eq!(JUNIOR_MINT_SEED, b"junior_mint");
        assert_eq!(WITHDRAW_RECEIPT_SEED, b"withdraw_receipt");
        assert_eq!(APR_FEED_SEED, b"apr_feed");
    }
}
