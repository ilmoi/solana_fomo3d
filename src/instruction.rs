use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize)]
pub enum FomoInstruction {
    InitiateGame(u8),
    InitiateRound,
    PurchaseKeys(PurchaseKeysParams),
    WithdrawSol,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct PurchaseKeysParams {
    pub sol_supplied: u128,
    pub team: u8,
}
