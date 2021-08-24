use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey,
};

use crate::math::common::TryAdd;
use crate::state::ROUND_INC_TIME_PER_KEY;
use crate::{
    math::common::{TryDiv, TryMul},
    processor::rng::pseudo_rng,
    state::RoundState,
};
use solana_program::sysvar::Sysvar;

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

pub fn round_ended(round_state: &RoundState) -> Result<bool, ProgramError> {
    let clock = Clock::get()?;
    //todo temp
    msg!(
        "round time left (s): {}",
        round_state.end_time - clock.unix_timestamp
    );
    Ok(round_state.end_time < clock.unix_timestamp)
}

/// New added delay = minimum of:
/// - number of keys purchased * time per key
/// - 24h from now
pub fn calc_new_delay(new_keys: u128) -> Result<u128, ProgramError> {
    let clock = Clock::get()?;
    let day_from_now = clock.unix_timestamp.try_add(24 * 60 * 60)?;
    let delay_based_on_keys = new_keys.try_mul(ROUND_INC_TIME_PER_KEY as u128)?;
    Ok(delay_based_on_keys.min(day_from_now as u128))
}
