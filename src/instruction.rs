use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize)]
pub enum GameInstruction {
    InitiateGame(u8),
    InitiateRound,
    PurchaseKeys(PurchaseKeysParams),
    WithdrawSol(WithdrawSolParams),
    EndRound,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct PurchaseKeysParams {
    pub sol_to_be_added: u128,
    pub team: u8,
    // pub affiliate_pk: [u8; 32],
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct WithdrawSolParams {
    // we want to let the user specify which round they'd like to withdraw for,
    // as they might have participated in more than one
    pub withdraw_for_round: u64,
}
