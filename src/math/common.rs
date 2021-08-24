use std::convert::TryFrom;

use solana_program::program_error::ProgramError;
use spl_math::approximations::sqrt;

use crate::{error::SomeError, math::precise::CheckedCeilDiv};

pub trait TrySub: Sized {
    fn try_sub(self, rhs: Self) -> Result<Self, ProgramError>;
}

pub trait TryAdd: Sized {
    fn try_add(self, rhs: Self) -> Result<Self, ProgramError>;
}

pub trait TryDiv<RHS>: Sized {
    fn try_floor_div(self, rhs: RHS) -> Result<Self, ProgramError>;
    fn try_ceil_div(self, rhs: RHS) -> Result<Self, ProgramError>;
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

pub trait TryRem: Sized {
    fn try_rem(self, rhs: Self) -> Result<Self, ProgramError>;
}

// --------------------------------------- u64

// impl TrySub for u64 {
//     fn try_sub(self, rhs: Self) -> Result<Self, ProgramError> {
//         self.checked_sub(rhs).ok_or(SomeError::BadError.into())
//     }
// }
//
// impl TryAdd for u64 {
//     fn try_add(self, rhs: Self) -> Result<Self, ProgramError> {
//         self.checked_add(rhs).ok_or(SomeError::BadError.into())
//     }
// }
//
// impl TryDiv<u64> for u64 {
//     fn try_div(self, rhs: u64) -> Result<Self, ProgramError> {
//         self.checked_div(rhs).ok_or(SomeError::BadError.into())
//     }
// }
//
// impl TryMul<u64> for u64 {
//     fn try_mul(self, rhs: u64) -> Result<Self, ProgramError> {
//         self.checked_mul(rhs).ok_or(SomeError::BadError.into())
//     }
// }
//
// impl TryPow<u32> for u64 {
//     fn try_pow(self, rhs: u32) -> Result<Self, ProgramError> {
//         self.checked_pow(rhs).ok_or(SomeError::BadError.into())
//     }
// }
//
// impl TrySqrt for u64 {
//     fn try_sqrt(self) -> Result<Self, ProgramError> {
//         sqrt(self).ok_or(SomeError::BadError.into())
//     }
// }
//
// impl TryRem for u64 {
//     fn try_rem(self, rhs: Self) -> Result<Self, ProgramError> {
//         self.checked_rem(rhs).ok_or(SomeError::BadError.into())
//     }
// }

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
    fn try_floor_div(self, rhs: u128) -> Result<Self, ProgramError> {
        self.checked_div(rhs).ok_or(SomeError::BadError.into())
    }
    fn try_ceil_div(self, rhs: u128) -> Result<Self, ProgramError> {
        let result = self
            .checked_ceil_div(rhs)
            .ok_or::<ProgramError>(SomeError::BadError.into())?;
        Ok(result.0)
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

impl TryRem for u128 {
    fn try_rem(self, rhs: Self) -> Result<Self, ProgramError> {
        self.checked_rem(rhs).ok_or(SomeError::BadError.into())
    }
}

// --------------------------------------- PreciseNumber

// impl TrySub for PreciseNumber {
//     fn try_sub(self, rhs: Self) -> Result<Self, ProgramError> {
//         self.checked_sub(&rhs).ok_or(SomeError::BadError.into())
//     }
// }
//
// impl TryAdd for PreciseNumber {
//     fn try_add(self, rhs: Self) -> Result<Self, ProgramError> {
//         self.checked_add(&rhs).ok_or(SomeError::BadError.into())
//     }
// }
//
// impl TryDiv<PreciseNumber> for PreciseNumber {
//     fn try_div(self, rhs: PreciseNumber) -> Result<Self, ProgramError> {
//         self.checked_div(&rhs).ok_or(SomeError::BadError.into())
//     }
// }
//
// impl TryMul<PreciseNumber> for PreciseNumber {
//     fn try_mul(self, rhs: PreciseNumber) -> Result<Self, ProgramError> {
//         self.checked_mul(&rhs).ok_or(SomeError::BadError.into())
//     }
// }
//
// impl TryPow<u128> for PreciseNumber {
//     fn try_pow(self, rhs: u128) -> Result<Self, ProgramError> {
//         self.checked_pow(rhs).ok_or(SomeError::BadError.into())
//     }
// }
//
// impl TrySqrt for PreciseNumber {
//     fn try_sqrt(self) -> Result<Self, ProgramError> {
//         self.sqrt().ok_or(SomeError::BadError.into())
//     }
// }

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

    #[test]
    fn test_floor_div() {
        //the easy (no remainder) case
        let x = 10_u128;
        let y = 5;
        let r = x.try_floor_div(y).unwrap();
        assert_eq!(r, 2);

        //<.5 case (2.2)
        let x = 11_u128;
        let y = 5;
        let r = x.try_floor_div(y).unwrap();
        assert_eq!(r, 2);

        //>.5 case (2.8)
        let x = 14_u128;
        let y = 5;
        let r = x.try_floor_div(y).unwrap();
        assert_eq!(r, 2);

        //.5 case
        let x = 5_u128;
        let y = 2;
        let r = x.try_floor_div(y).unwrap();
        assert_eq!(r, 2);
    }

    #[test]
    fn test_ceil_div() {
        //the easy (no remainder) case
        let x = 10_u128;
        let y = 5;
        let r = x.try_ceil_div(y).unwrap();
        assert_eq!(r, 2);

        //<.5 case (2.2)
        let x = 11_u128;
        let y = 5;
        let r = x.try_ceil_div(y).unwrap();
        assert_eq!(r, 3);

        //>.5 case (2.8)
        let x = 14_u128;
        let y = 5;
        let r = x.try_ceil_div(y).unwrap();
        assert_eq!(r, 3);

        //.5 case
        let x = 5_u128;
        let y = 2;
        let r = x.try_ceil_div(y).unwrap();
        assert_eq!(r, 3);
    }
}
