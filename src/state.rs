use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

// --------------------------------------- game state

pub const GAME_STATE_SIZE: usize = 8 * 4 + 1;
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct GameState {
    pub round_id: u64,        //round id number / total rounds that have happened
    pub round_init_time: u64, //in seconds, wait time before a new round begins, after previous ended
    pub round_inc_time: u64,  //in seconds, how much each key purchase increases the time
    pub round_max_time: u64,  //in seconds, max timer time
    pub version: u8,
}

pub const ROUND_INIT_TIME: u64 = 1 * 60 * 60; //1h
pub const ROUND_INC_TIME: u64 = 30; //30s
pub const ROUND_MAX_TIME: u64 = 24 * 60 * 60; //24h

// --------------------------------------- fees & teams

pub const FEE_SPLIT_SIZE: usize = 16;
// when a key is purchased the fees are split between 1)next round, 2)f3d players, 3)p3d holders.
// (1) can be deduced as 100 - (2)f3d - (3)p3d
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct FeeSplit {
    f3d: u64,
    p3d: u64,
}

pub const POT_SPLIT_SIZE: usize = 16;
// when the round is over the pot is split between 1)next round, 2)f3d players, 3)p3d holders.
// (1) can be deduced as 100 - (2)f3d - (3)p3d
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct PotSplit {
    f3d: u64,
    p3d: u64,
}

pub const TEAM_SIZE: usize = 1 + 16 + 16; //extra 1 for the enum
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub enum Team {
    Init(FeeSplit, PotSplit), //used to init a fresh round
    Whale(FeeSplit, PotSplit),
    Bear(FeeSplit, PotSplit),
    Snek(FeeSplit, PotSplit),
    Bull(FeeSplit, PotSplit),
}

pub const INIT_FEE_SPLIT: FeeSplit = FeeSplit { f3d: 0, p3d: 0 }; //used to init a fresh round
pub const WHALE_FEE_SPLIT: FeeSplit = FeeSplit { f3d: 30, p3d: 6 };
pub const BEAR_FEE_SPLIT: FeeSplit = FeeSplit { f3d: 43, p3d: 0 };
pub const SNEK_FEE_SPLIT: FeeSplit = FeeSplit { f3d: 56, p3d: 10 };
pub const BULL_FEE_SPLIT: FeeSplit = FeeSplit { f3d: 43, p3d: 8 };

pub const INIT_POT_SPLIT: PotSplit = PotSplit { f3d: 0, p3d: 0 }; //used to init a fresh round
pub const WHALE_POT_SPLIT: PotSplit = PotSplit { f3d: 15, p3d: 10 };
pub const BEAR_POT_SPLIT: PotSplit = PotSplit { f3d: 25, p3d: 0 };
pub const SNEK_POT_SPLIT: PotSplit = PotSplit { f3d: 20, p3d: 20 };
pub const BULL_POT_SPLIT: PotSplit = PotSplit { f3d: 30, p3d: 10 };

// --------------------------------------- round

pub type UnixTimestamp = i64;

pub const ROUND_STATE_SIZE: usize = 8 * 11 + 1 + 32 + TEAM_SIZE;
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct RoundState {
    pub round_id: u64,
    pub lead_player_pk: Pubkey,
    pub lead_player_team: Team,
    pub start_time: UnixTimestamp, //the time the round starts / has started
    pub end_time: UnixTimestamp,   //the time the round ends / has ended
    pub ended: bool,               //whether the round has ended
    pub accum_keys: u64,
    pub accum_sol_pot: u64,
    pub accum_f3d_share: u64,
    pub accum_p3d_share: u64,
    pub accum_community_share: u64,
    pub accum_next_round_share: u64,
    pub accum_airdrop_share: u64, //person who gets the airdrop wins part of this pot
    pub airdrop_tracker: u64,     //increment each time a qualified tx occurs
}

// --------------------------------------- player

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct PlayerState {
    pub player_pk: Pubkey,
    pub accum_winnings: u64,    //vault for the final sum if the user wins
    pub accum_f3d: u64,         //vault for dividends from key ownership
    pub accum_aff: u64,         //vault for affiliate dividends (for referrals)
    pub last_round_id: u64,     //last round the user participated in
    pub last_affiliate_id: u64, //whoever referred the user (todo I think)
}

// --------------------------------------- player x round

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct PlayerRound {
    pub player_pk: Pubkey,
    pub round_id: u64,
    pub keys: u64,                //number of keys owned by the user
    pub accum_sol_added: u64,     //amount of SOL the player has added to round (used as limiter)
    pub accum_sol_withdrawn: u64, //dividends already PAID OUT to user
}

// --------------------------------------- other stuff

//how to get size:
//let t = Team::Bear(BEAR_FEE_SPLIT, BEAR_POT_SPLIT);
//let t_size = t.try_to_vec().unwrap().len();
//msg!("team size is {}", t_size);

//todo do I need an is_initialized for each of these?
// - eg lending has a cool fn assert_initialized

//todo math - should I be using u64 or something else for all the wSol operations?
