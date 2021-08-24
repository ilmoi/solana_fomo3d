use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

use crate::processor::util::is_zero;
use std::str::FromStr;

pub type UnixTimestamp = i64;

// --------------------------------------- game state

pub const GAME_STATE_SIZE: usize = (8 * 4) + 1 + (32 * 4);
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct GameState {
    pub round_id: u64,        //round id number / total rounds that have happened
    pub round_init_time: i64, //in seconds, wait time before a new round begins, after previous ended
    pub round_inc_time: i64,  //in seconds, how much each key purchase increases the time
    pub round_max_time: i64,  //in seconds, max timer time
    pub version: u8,
    pub mint: Pubkey,
    //privileged accounts
    pub game_creator: Pubkey,
    pub community_wallet: Pubkey,
    pub p3d_wallet: Pubkey,
}

// pub const ROUND_INIT_TIME: i64 = 1 * 60 * 60; //1h
pub const ROUND_INIT_TIME: i64 = 2; //todo temp

// pub const ROUND_INC_TIME: i64 = 30; //30s
pub const ROUND_INC_TIME: i64 = 2; //todo temp
pub const ROUND_MAX_TIME: i64 = 24 * 60 * 60; //24h

// --------------------------------------- fees & teams

pub const FEE_SPLIT_SIZE: usize = 2;
// when a key is purchased the fees are split between 1)next round, 2)f3d players, 3)p3d holders.
// (1) can be deduced as 86 - (2)f3d - (3)p3d
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct FeeSplit {
    pub f3d: u8,
    pub p3d: u8,
}

pub const POT_SPLIT_SIZE: usize = 2;
// when the round is over the pot is split between 1)next round, 2)f3d players, 3)p3d holders.
// (1) can be deduced as 50 - (2)f3d - (3)p3d
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct PotSplit {
    pub f3d: u8,
    pub p3d: u8,
}

pub const TEAM_SIZE: usize = 1;
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub enum Team {
    Whale,
    Bear,
    Snek,
    Bull,
}

pub const WHALE_FEE_SPLIT: FeeSplit = FeeSplit { f3d: 30, p3d: 6 };
pub const BEAR_FEE_SPLIT: FeeSplit = FeeSplit { f3d: 43, p3d: 0 };
pub const SNEK_FEE_SPLIT: FeeSplit = FeeSplit { f3d: 56, p3d: 10 };
pub const BULL_FEE_SPLIT: FeeSplit = FeeSplit { f3d: 43, p3d: 8 };

pub const WHALE_POT_SPLIT: PotSplit = PotSplit { f3d: 15, p3d: 10 };
pub const BEAR_POT_SPLIT: PotSplit = PotSplit { f3d: 25, p3d: 0 };
pub const SNEK_POT_SPLIT: PotSplit = PotSplit { f3d: 20, p3d: 20 };
pub const BULL_POT_SPLIT: PotSplit = PotSplit { f3d: 30, p3d: 10 };

pub const SOL_BY_TEAM_SIZE: usize = 16 * 4;
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct SolByTeam {
    pub whale: u128,
    pub bear: u128,
    pub snek: u128,
    pub bull: u128,
}

// --------------------------------------- round

pub const ROUND_STATE_SIZE: usize =
    8 + 32 + TEAM_SIZE + (8 * 2) + 1 + SOL_BY_TEAM_SIZE + (13 * 16) + 8;
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct RoundState {
    pub round_id: u64,
    //lead player
    pub lead_player_pk: Pubkey,
    pub lead_player_team: Team,
    //timing
    pub start_time: UnixTimestamp, //the time the round starts / has started
    pub end_time: UnixTimestamp,   //the time the round ends / has ended
    pub ended: bool,               //whether the round has ended
    //totals
    pub accum_keys: u128,
    pub accum_sol_pot: u128, //in lamports
    pub accum_sol_by_team: SolByTeam,
    //shares
    pub accum_community_share: u128,
    pub accum_airdrop_share: u128, //person who gets the airdrop wins part of this pot
    pub accum_next_round_share: u128,
    pub accum_aff_share: u128, //sum of all affiliate shares paid out to users (used for checks & balances)
    pub accum_p3d_share: u128,
    pub accum_f3d_share: u128, //sum of all f3d shares paid out to users (used for checks & balances)
    pub still_in_play: u128,
    pub final_prize_share: u128, //will be filled when round ends
    //withdrawal history (used to offset any future attempts)
    pub withdrawn_com: u128,
    pub withdrawn_next_round: u128,
    pub withdrawn_p3d: u128,
    //airdrop
    pub airdrop_tracker: u64, //increment each time a qualified tx occurs
}

// --------------------------------------- player x round

pub const PLAYER_ROUND_STATE_SIZE: usize = 32 + 8 + 32 + (7 * 16);
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct PlayerRoundState {
    pub player_pk: Pubkey,
    pub round_id: u64,
    pub last_affiliate_pk: Pubkey, //last person to refer the player
    //totals
    pub accum_keys: u128,      //number of keys owned by the user
    pub accum_sol_added: u128, //amount of SOL the player has added to round (used as limiter)
    //shares (available for withdrawal to the user)
    //NOTE: f3d share is calculated dynamically at the time of withdrawal due to constantly changing key ratio
    pub accum_winnings: u128, //accumulated winnings from 1)the airdrop lottery, 2)the final prize
    pub accum_aff: u128,      //accumulated affiliate dividends
    //withdrawal history (used to offset any future attempts)
    pub withdrawn_winnings: u128,
    pub withdrawn_aff: u128,
    pub withdrawn_f3d: u128,
}

impl PlayerRoundState {
    pub fn has_affiliate(&self) -> bool {
        !is_zero(&self.last_affiliate_pk.to_bytes())
    }
}

// --------------------------------------- other stuff

//how to get size:
//let t = Team::Bear(BEAR_FEE_SPLIT, BEAR_POT_SPLIT);
//let t_size = t.try_to_vec().unwrap().len();
//msg!("team size is {}", t_size);

//todo do I need an is_initialized for each of these?
// - eg lending has a cool fn assert_initialized

//todo math - should I be using u64 or something else for all the wSol operations?
