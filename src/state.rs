use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct Fomo3dState {
    pub round_id: u64,        //round id number / total rounds that have happened
    pub round_init_time: u64, //in seconds, wait time before a new round begins, after previous ended
    pub round_inc_time: u64,  //in seconds, how much each key purchase increases the time
    pub round_max_time: u64,  //in seconds, max timer time
    // pub pot: Pubkey,
    pub state_bump: u8,
    pub pot_bump: u8,
    //34
}

/// 2 types of splits exist:
///  1) when a key is purchased, the fees are split between the pot / f3d holders / p3d holders
///  2) when the round is over, the proceeds are split between the next round / f3d holders / p3d holders
/// f3d and p3d are noted below, the 3rd element can be deduced as 100 - f3d split - p3d split.
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct FeeSplit {
    f3d: u64,
    p3d: u64,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct PotSplit {
    f3d: u64,
    p3d: u64,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub enum Team {
    Whale(FeeSplit, PotSplit),
    Bear(FeeSplit, PotSplit),
    Snek(FeeSplit, PotSplit),
    Bull(FeeSplit, PotSplit),
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct RoundState {
    pub lead_player_id: u64,
    pub lead_player_team: Team,
    pub start_time: u64, //the time the round starts / has started
    pub end_time: u64,   //the time the round ends / has ended
    pub ended: bool,     //whether the round has ended
    pub total_keys: u64,
    pub total_sol: u64,
    pub airdrop_amount: u64, //person who gets the airdrop wins part of this pot
    pub airdrop_tracker: u64, //increment each time a qualified tx occurs
    pub mask: u64,           //profits that go to users, per key
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct PlayerState {
    pub player_id: u64,
    pub player_name: String,
    pub vault_win: u64,         //vault for the final sum if the user wins
    pub vault_f3d: u64,         //vault for dividends from key ownership
    pub vault_aff: u64,         //vault for affiliate dividends (for referrals)
    pub last_round_id: u64,     //last round the user participated in
    pub last_affiliate_id: u64, //whoever referred the user (todo I think)
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct PlayerRound {
    pub player_id: u64,
    pub round_id: u64,
    pub sol: u64,  //amount of SOL the player has added to round (used as limiter)
    pub keys: u64, //number of keys owned by the user
    pub mask: u64, //dividends already PAID OUT to user
}

//todo do I need an is_initialized for each of these?
// - eg lending has a cool fn assert_initialized
