use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize)]
pub enum FomoInstruction {
    /// Initializes fomo3d
    Initialize(u8),
    /// Allows the user to purchase keys
    PurchaseKeys(u64),
    /// temp ix to test moving funds out
    PayOut(u8),
}
