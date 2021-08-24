use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, program_pack::Pack, pubkey::Pubkey,
};
use spl_token::state::Account;

use crate::{
    math::common::{TryDiv, TryMul},
    processor::rng::pseudo_rng,
    state::RoundState,
};

/// The original math for this is unnecessary convoluted and we decided to ignore it.
/// Ultimately this comes down to a simple equation: (player's keys / total keys) * total f3d earnings.
/// That's the approach taken below. For anyone interested in original math follow these links:
/// https://gist.github.com/ilmoi/4daad0d6e9730cc6af833c065a95b717#file-fomo-sol-L1533
/// https://gist.github.com/ilmoi/4daad0d6e9730cc6af833c065a95b717#file-fomo-sol-L1125
pub fn calculate_player_f3d_share(
    player_keys: u128,
    total_keys: u128,
    accum_f3d: u128,
) -> Result<u128, ProgramError> {
    //in theory, there might be unaccounted dust left here.
    //eg player1 keys = 333, player2 keys =  total keys = 1000, f3t pot = 100
    //then player1 will get 33, player2 will get 66, and 1 will be left as dust
    //in practice, however, to account for it would have to coordinate all withdrawals by all players
    //which of course isn't possible. So it will just be left in the protocol
    player_keys.try_mul(accum_f3d)?.try_floor_div(total_keys)
}

//todo problem with this check - what if someone randomly sends tokens to the pot? then the amount won't match ofc
/// Checks whether actual funds in the pot equate to total of all the parties' shares.
pub fn verify_round_state(round_state: &RoundState, pot_info: &AccountInfo) -> ProgramResult {
    let actual_money_in_pot = Account::unpack(&pot_info.data.borrow())?.amount;
    let supposed_money_in_pot = round_state.accum_community_share
        + round_state.accum_airdrop_share
        + round_state.accum_next_round_share
        + round_state.accum_aff_share
        + round_state.accum_p3d_share
        + round_state.accum_f3d_share
        + round_state.accum_prize_share;
    msg!("{}, {}", actual_money_in_pot as u128, supposed_money_in_pot);
    assert_eq!(actual_money_in_pot as u128, supposed_money_in_pot);
    Ok(())
}

pub fn airdrop_winner(
    player_pk: &Pubkey,
    clock: &Clock,
    airdrop_tracker: u64,
) -> Result<bool, ProgramError> {
    let lottery_ticket = pseudo_rng(player_pk, clock)?;
    Ok(lottery_ticket < airdrop_tracker as u128)
}

pub fn account_exists(acc: &AccountInfo) -> bool {
    let does_not_exist = **acc.lamports.borrow() == 0 || acc.data_is_empty();
    !does_not_exist
}

pub fn is_zero(buf: &[u8]) -> bool {
    let (prefix, aligned, suffix) = unsafe { buf.align_to::<u128>() };

    prefix.iter().all(|&x| x == 0)
        && suffix.iter().all(|&x| x == 0)
        && aligned.iter().all(|&x| x == 0)
}
