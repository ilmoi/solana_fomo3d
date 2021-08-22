use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize)]
pub enum FomoInstruction {
    InitiateGame(u8),
    InitiateRound,
    PurchaseKeys(u64),
    WithdrawSol,
}
