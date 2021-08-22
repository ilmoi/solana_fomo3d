use solana_program::program_error::{ProgramError, PrintProgramError};
use solana_program::{decode_error::DecodeError, msg};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use thiserror::Error;


#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum SomeError {
    //some bad error
    #[error("some bad error happened")]
    BadError,
}

// --------------------------------------- so that fn return type is happy

impl From<SomeError> for ProgramError {
    fn from(e: SomeError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

// --------------------------------------- to be able to print the error

impl<T> DecodeError<T> for SomeError {
    fn type_of() -> &'static str {
        "ouch some error happened"
    }
}

impl PrintProgramError for SomeError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            SomeError::BadError => msg!("ouch bad error happened"),
        }
    }
}