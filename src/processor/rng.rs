use crate::error::SomeError;
use crate::math::common::TryRem;
use solana_program::clock::Clock;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::sysvar::Sysvar;
use std::convert::{TryFrom, TryInto};

/// Generates a pseudo-random number in the [0,1000) range.
/// (!) NOT A REAL RANDOM NUMBER GENERATOR
///     Real rng would come from an off-chain oracle, which currently doesn't exist on Solana.
///     This rng is predictable and is purely used for demonstration purposes.
///     In fact the original Fomo3D protocol has a security vulnerability due to using an on-chain rng -
///     https://www.reddit.com/r/ethereum/comments/916xni/how_to_pwn_fomo3d_a_beginners_guide/
pub fn pseudo_rng(player_pk: &Pubkey, clock: &Clock) -> Result<u128, ProgramError> {
    let mut data = vec![];

    let local = player_pk.to_bytes();
    let temporal = clock.unix_timestamp as u64
        + clock.epoch
        + clock.slot
        + clock.epoch_start_timestamp as u64
        + clock.leader_schedule_epoch;

    data.extend_from_slice(&local);
    data.extend_from_slice(&temporal.to_le_bytes());

    let hash = solana_program::keccak::hash(&data).to_bytes();
    let short_hash = &hash[..16];
    let hash_int = u128::from_le_bytes(short_hash.try_into().unwrap());
    hash_int.try_rem(1000)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entrypoint::process_instruction;
    use solana_program_test::*;

    #[tokio::test]
    async fn test_pseudo_rng() {
        let program_id = Pubkey::new_unique();
        let (mut banks_client, payer, recent_blockhash) = ProgramTest::new(
            "bpf_program_template",
            program_id,
            processor!(process_instruction),
        )
        .start()
        .await;
        let clock = banks_client.get_clock().await.unwrap();
        let player_pk = Pubkey::new_unique();
        let result = pseudo_rng(&player_pk, &clock).unwrap();
        assert!(0 <= result && result < 1000);
    }
}
