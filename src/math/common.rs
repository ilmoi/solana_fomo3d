use crate::error::SomeError;
use crate::math::precise::PreciseNumber;
use solana_program::program_error::ProgramError;
use spl_math::approximations::sqrt;
use std::convert::TryFrom;

pub trait TrySub: Sized {
    fn try_sub(self, rhs: Self) -> Result<Self, ProgramError>;
}

pub trait TryAdd: Sized {
    fn try_add(self, rhs: Self) -> Result<Self, ProgramError>;
}

pub trait TryDiv<RHS>: Sized {
    fn try_div(self, rhs: RHS) -> Result<Self, ProgramError>;
}

pub trait TryMul<RHS>: Sized {
    fn try_mul(self, rhs: RHS) -> Result<Self, ProgramError>;
}

pub trait TryPow<RHS>: Sized {
    fn try_pow(self, rhs: RHS) -> Result<Self, ProgramError>;
}

pub trait TrySqrt: Sized {
    fn try_sqrt(self) -> Result<Self, ProgramError>;
}

pub trait TryCast<Into>: Sized {
    fn try_cast(self) -> Result<Into, ProgramError>;
}

// --------------------------------------- u64

impl TrySub for u64 {
    fn try_sub(self, rhs: Self) -> Result<Self, ProgramError> {
        self.checked_sub(rhs).ok_or(SomeError::BadError.into())
    }
}

impl TryAdd for u64 {
    fn try_add(self, rhs: Self) -> Result<Self, ProgramError> {
        self.checked_add(rhs).ok_or(SomeError::BadError.into())
    }
}

impl TryDiv<u64> for u64 {
    fn try_div(self, rhs: u64) -> Result<Self, ProgramError> {
        self.checked_div(rhs).ok_or(SomeError::BadError.into())
    }
}

impl TryMul<u64> for u64 {
    fn try_mul(self, rhs: u64) -> Result<Self, ProgramError> {
        self.checked_mul(rhs).ok_or(SomeError::BadError.into())
    }
}

impl TryPow<u32> for u64 {
    fn try_pow(self, rhs: u32) -> Result<Self, ProgramError> {
        self.checked_pow(rhs).ok_or(SomeError::BadError.into())
    }
}

impl TrySqrt for u64 {
    fn try_sqrt(self) -> Result<Self, ProgramError> {
        sqrt(self).ok_or(SomeError::BadError.into())
    }
}

// --------------------------------------- u128

impl TrySub for u128 {
    fn try_sub(self, rhs: Self) -> Result<Self, ProgramError> {
        self.checked_sub(rhs).ok_or(SomeError::BadError.into())
    }
}

impl TryAdd for u128 {
    fn try_add(self, rhs: Self) -> Result<Self, ProgramError> {
        self.checked_add(rhs).ok_or(SomeError::BadError.into())
    }
}

impl TryDiv<u128> for u128 {
    fn try_div(self, rhs: u128) -> Result<Self, ProgramError> {
        self.checked_div(rhs).ok_or(SomeError::BadError.into())
    }
}

impl TryMul<u128> for u128 {
    fn try_mul(self, rhs: u128) -> Result<Self, ProgramError> {
        self.checked_mul(rhs).ok_or(SomeError::BadError.into())
    }
}

impl TryPow<u32> for u128 {
    fn try_pow(self, rhs: u32) -> Result<Self, ProgramError> {
        self.checked_pow(rhs).ok_or(SomeError::BadError.into())
    }
}

impl TrySqrt for u128 {
    fn try_sqrt(self) -> Result<Self, ProgramError> {
        sqrt(self).ok_or(SomeError::BadError.into())
    }
}

impl TryCast<u64> for u128 {
    fn try_cast(self) -> Result<u64, ProgramError> {
        u64::try_from(self).map_err(|_| SomeError::BadError.into())
    }
}

// --------------------------------------- PreciseNumber

impl TrySub for PreciseNumber {
    fn try_sub(self, rhs: Self) -> Result<Self, ProgramError> {
        self.checked_sub(&rhs).ok_or(SomeError::BadError.into())
    }
}

impl TryAdd for PreciseNumber {
    fn try_add(self, rhs: Self) -> Result<Self, ProgramError> {
        self.checked_add(&rhs).ok_or(SomeError::BadError.into())
    }
}

impl TryDiv<PreciseNumber> for PreciseNumber {
    fn try_div(self, rhs: PreciseNumber) -> Result<Self, ProgramError> {
        self.checked_div(&rhs).ok_or(SomeError::BadError.into())
    }
}

impl TryMul<PreciseNumber> for PreciseNumber {
    fn try_mul(self, rhs: PreciseNumber) -> Result<Self, ProgramError> {
        self.checked_mul(&rhs).ok_or(SomeError::BadError.into())
    }
}

impl TryPow<u128> for PreciseNumber {
    fn try_pow(self, rhs: u128) -> Result<Self, ProgramError> {
        self.checked_pow(rhs).ok_or(SomeError::BadError.into())
    }
}

impl TrySqrt for PreciseNumber {
    fn try_sqrt(self) -> Result<Self, ProgramError> {
        self.sqrt().ok_or(SomeError::BadError.into())
    }
}

// --------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u128_to_u64() {
        let big = (u64::MAX) as u128;
        let small = big.try_cast();
        assert!(small.is_ok());
        assert_eq!(small.unwrap(), u64::MAX);

        let too_big = ((u64::MAX) as u128) + 1;
        let small = too_big.try_cast();
        assert!(small.is_err());
    }
}
