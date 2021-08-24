use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize)]
pub enum GameInstruction {
    //todo list accounts
    //0
    InitiateGame(u8),
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
pub struct PurchaseKeysParams {
    pub sol_to_be_added: u128,
    pub team: u8,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct WithdrawParams {
    //user should be able to specify which round they want to withdraw for
    pub withdraw_for_round: u64,
}
