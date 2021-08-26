use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize)]
pub enum GameInstruction {
    //todo list accounts
    //0
    InitiateGame(InitGameParams),
    //1
    InitiateRound,
    //2
    PurchaseKeys(PurchaseKeysParams),
    //3
    WithdrawSol(WithdrawParams),
    //4
    EndRound,
    //5
    WithdrawCommunityRewards(WithdrawParams),
    //6
    WithdrawP3DRewards(WithdrawParams),
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct InitGameParams {
    pub version: u64,
    //time (in seconds) for the initial window when a new round starts
    //in original Fomo3D: 1h
    pub round_init_time: i64,
    //time (in seconds) by which each key purchase bumps round end time
    //in original Fomo3D: 30s
    pub round_inc_time_per_key: i64,
    //time (in seconds) for max possible window
    //in original Fomo3D: 24h
    pub round_max_time: i64,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct PurchaseKeysParams {
    pub sol_to_be_added: u128,
    pub team: u8,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct WithdrawParams {
    //user should be able to specify which round they want to withdraw for
    pub withdraw_for_round: u64,
}
