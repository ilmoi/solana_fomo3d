use crate::error::GameError;
use crate::math::common::TryAdd;
use crate::processor::util::load_pk;
use crate::state::StateType::{GameStateTypeV1, PlayerRoundStateTypeV1, RoundStateTypeV1};
use crate::state::{GameState, PlayerRoundState, RoundState};
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::{msg, pubkey::Pubkey};

// --------------------------------------- state

/// Checks whether actual funds in the pot equate to total of all the parties' shares.
/// NOTE: considered comparing vs actual money in pot but problems arise:
///  - what if someone randommly sends money to pot
///  - what if one of the players withdraws their affiliate share
///    (we would have to scape every user account's state to adjust expectations)
pub fn verify_round_state(round_state: &RoundState) -> ProgramResult {
    let actual_money_in_pot = round_state.accum_sol_pot;
    let supposed_money_in_pot = round_state
        .accum_community_share
        .try_add(round_state.accum_airdrop_share)?
        .try_add(round_state.accum_next_round_share)?
        .try_add(round_state.accum_aff_share)?
        .try_add(round_state.accum_p3d_share)?
        .try_add(round_state.accum_f3d_share)?
        .try_add(round_state.still_in_play)?
        .try_add(round_state.final_prize_share)?;
    assert_eq!(actual_money_in_pot, supposed_money_in_pot);
    Ok(())
}

pub trait VerifyType {
    fn verify_type(&self) -> ProgramResult;
}
impl VerifyType for GameState {
    fn verify_type(&self) -> ProgramResult {
        if self.TYPE != GameStateTypeV1 {
            return Err(GameError::InvalidStateType.into());
        }
        Ok(())
    }
}
impl VerifyType for RoundState {
    fn verify_type(&self) -> ProgramResult {
        if self.TYPE != RoundStateTypeV1 {
            return Err(GameError::InvalidStateType.into());
        }
        Ok(())
    }
}
impl VerifyType for PlayerRoundState {
    fn verify_type(&self) -> ProgramResult {
        if self.TYPE != PlayerRoundStateTypeV1 {
            return Err(GameError::InvalidStateType.into());
        }
        Ok(())
    }
}

// --------------------------------------- ownership

pub enum Owner {
    SystemProgram,
    TokenProgram,
    NativeLoader,
    BPFLoader,
    Sysvar,
    Other(Pubkey),
    None,
}

pub fn verify_account_ownership(
    accounts: &[AccountInfo],
    expected_owners: &[Owner],
) -> ProgramResult {
    for (i, account) in accounts.iter().enumerate() {
        let expected_owner = match &expected_owners[i] {
            Owner::SystemProgram => solana_program::system_program::id(),
            Owner::TokenProgram => spl_token::id(),
            Owner::NativeLoader => load_pk("NativeLoader1111111111111111111111111111111")?,
            Owner::BPFLoader => load_pk("BPFLoader2111111111111111111111111111111111")?,
            Owner::Sysvar => load_pk("Sysvar1111111111111111111111111111111111111")?,
            Owner::Other(pk) => *pk,
            Owner::None => {
                //no need to check owner for this account
                continue;
            }
        };

        if *account.owner != expected_owner {
            msg!(
                "Account {} is expected to be owned by {}, but is actually owned by {}",
                account.key,
                expected_owner,
                account.owner,
            );
            return Err(GameError::InvalidOwner.into());
        }
    }
    Ok(())
}

// --------------------------------------- signature

pub fn verify_is_signer(account: &AccountInfo) -> ProgramResult {
    if !account.is_signer {
        return Err(GameError::MissingSignature.into());
    }
    Ok(())
}

// --------------------------------------- CPI

pub fn verify_token_program(token_program: &AccountInfo) -> ProgramResult {
    if token_program.key != &spl_token::id() {
        return Err(GameError::InvalidTokenProgram.into());
    }
    Ok(())
}
