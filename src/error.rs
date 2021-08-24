use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use solana_program::{
    decode_error::DecodeError,
    msg,
    program_error::{PrintProgramError, ProgramError},
};
use thiserror::Error;

#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum GameError {
    //math
    #[error("General calculation failure due to overflow or underflow")]
    CalculationFailure,
    #[error("Conversion to u64 failed with an overflow or underflow")]
    ConversionFailure,
    #[error("Supplied amount is above threshold")]
    AboveThreshold,
    #[error("Supplied amount is below floor")]
    BelowFloor,

    //spl
    #[error("Failed to invoke the SPL Token Program")]
    TokenProgramInvocationFailure,
    #[error("Failed to match mint of provided token account")]
    MintMatchFailure,

    //pda/accounts
    #[error("Failed to unpack account")]
    UnpackingFailure,
    #[error("Failed to match the provided PDA with that internally derived")]
    PDAMatchFailure,

    //general
    #[error("Invalid owner passed")]
    InvalidOwner,
    #[error("Game/round account already initialized")]
    AlreadyInitialized,
    #[error("Missing an expected signature")]
    MissingSignature,
    #[error("An additional account was expected")]
    MissingAccount,
    #[error("Wrong account has been passed")]
    WrongAccount,
    // #[error("Invalid instruction")]
    // InvalidInstruction,

    //round
    #[error("Previous round hasn't yet ended")]
    NotYetEnded,
    #[error("Previous round has already ended")]
    AlreadyEnded,
}

// --------------------------------------- so that fn return type is happy

impl From<GameError> for ProgramError {
    fn from(e: GameError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

// --------------------------------------- to be able to print the error

impl<T> DecodeError<T> for GameError {
    fn type_of() -> &'static str {
        "ouch some error happened"
    }
}

impl PrintProgramError for GameError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            //math
            GameError::CalculationFailure => {
                msg!("General calculation failure due to overflow or underflow")
            }
            GameError::ConversionFailure => {
                msg!("Conversion to u64 failed with an overflow or underflow")
            }
            GameError::AboveThreshold => msg!("Supplied amount is above threshold"),
            GameError::BelowFloor => msg!("Supplied amount is below floor"),
            //spl
            GameError::TokenProgramInvocationFailure => {
                msg!("Failed to invoke the SPL Token Program")
            }
            GameError::MintMatchFailure => msg!("Failed to match mint of provided token account"),
            //pda/accounts
            GameError::UnpackingFailure => msg!("Failed to unpack account"),
            GameError::PDAMatchFailure => {
                msg!("Failed to match the provided PDA with that internally derived")
            }
            //general
            GameError::InvalidOwner => msg!("Invalid owner passed"),
            GameError::AlreadyInitialized => msg!("Game/round account already initialized"),
            GameError::MissingSignature => msg!("Missing an expected signature"),
            GameError::MissingAccount => msg!("An additional account was expected"),
            GameError::WrongAccount => msg!("Wrong account has been passed"),
            // GameError::InvalidInstruction => msg!("Invalid instruction"),
            //round
            GameError::NotYetEnded => msg!("Previous round hasn't yet ended"),
            GameError::AlreadyEnded => msg!("Previous round has already ended"),
        }
    }
}
