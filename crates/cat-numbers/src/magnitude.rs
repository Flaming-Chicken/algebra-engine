//! Magnitude trait mapping signed, unsigned, floating-point, and arbitrary-precision
//! numeric types to their non-negative magnitude type without overflowing minimum signed values.
//!
//! Salvaged from archived `num_extended` into `cat-numbers`.

use num_bigint::{BigInt, BigUint};
use num_traits::{Num, Zero};

#[doc(hidden)]
macro_rules! magnitude_prim_int {
    ($u:ty) => {
        impl Magnitude for $u {
            type Output = $u;

            fn magnitude(self) -> <Self as Magnitude>::Output {
                self
            }
        }
    };

    ($u:ty, $i:ty) => {
        impl Magnitude for $i {
            type Output = $u;

            fn magnitude(self) -> <Self as Magnitude>::Output {
                self.abs_diff(<$i as Zero>::zero())
            }
        }
    };
}

/// Computes the unsigned/non-negative magnitude of a numeric value.
///
/// Unlike `Signed::abs`, signed primitive implementations return the corresponding unsigned
/// type (e.g., `i8::MIN.magnitude() == 128u8`) and therefore cannot overflow on `MIN`.
pub trait Magnitude: Num {
    /// Non-negative magnitude representation for `Self`.
    type Output: Num;

    /// Returns the non-negative magnitude of `self`.
    fn magnitude(self) -> <Self as Magnitude>::Output;
}

magnitude_prim_int!(u8);
magnitude_prim_int!(u16);
magnitude_prim_int!(u32);
magnitude_prim_int!(u64);
magnitude_prim_int!(u128);

magnitude_prim_int!(u8, i8);
magnitude_prim_int!(u16, i16);
magnitude_prim_int!(u32, i32);
magnitude_prim_int!(u64, i64);
magnitude_prim_int!(u128, i128);

magnitude_prim_int!(usize);
magnitude_prim_int!(usize, isize);

impl Magnitude for f32 {
    type Output = f32;

    fn magnitude(self) -> <Self as Magnitude>::Output {
        self.abs()
    }
}

impl Magnitude for f64 {
    type Output = f64;

    fn magnitude(self) -> <Self as Magnitude>::Output {
        self.abs()
    }
}

magnitude_prim_int!(BigUint);

impl Magnitude for BigInt {
    type Output = BigUint;

    fn magnitude(self) -> <Self as Magnitude>::Output {
        BigInt::magnitude(&self).clone()
    }
}

#[cfg(test)]
mod tests {
    use num_bigint::{BigInt, BigUint, Sign};
    use num_traits::Zero;

    use super::*;

    #[test]
    fn unsignedprimitive() -> Result<(), String> {
        let eight: u8 = u8::MAX;
        let size: usize = usize::MAX;
        let big: u128 = u128::MAX;

        let mag_eight: u8 = eight.magnitude();
        let mag_size: usize = size.magnitude();
        let mag_big: u128 = big.magnitude();

        let mut result = String::new();

        if eight != mag_eight {
            result += "u8 failed. ";
        }
        if size != mag_size {
            result += "usize failed. ";
        }
        if big != mag_big {
            result += "u128 failed. ";
        }

        if !result.is_empty() {
            Err(result)
        } else {
            Ok(())
        }
    }

    #[test]
    fn signedprimitive() -> Result<(), String> {
        let eight: i8 = i8::MIN;
        let thirtytwo: i32 = i32::MIN;
        let sixtyfour: i64 = i64::MIN;
        let big: i128 = i128::MIN;

        let mag_eight: u8 = eight.magnitude();
        let mag_thirtytwo: u32 = thirtytwo.magnitude();
        let mag_sixtyfour: u64 = sixtyfour.magnitude();
        let mag_big: u128 = big.magnitude();

        let mut result = String::new();

        if 128u8 != mag_eight {
            result += "i8 failed. ";
        }
        if 2_147_483_648u32 != mag_thirtytwo {
            result += "i32 failed. ";
        }
        if 9_223_372_036_854_775_808u64 != mag_sixtyfour {
            result += "i64 failed. ";
        }
        if 170_141_183_460_469_231_731_687_303_715_884_105_728u128 != mag_big {
            result += "i128 failed. ";
        }

        if !result.is_empty() {
            Err(result)
        } else {
            Ok(())
        }
    }

    #[test]
    fn floatprimitive() -> Result<(), String> {
        let float_min: f32 = f32::MIN;
        let float_inf: f32 = f32::NEG_INFINITY;
        let float_nan: f32 = f32::NAN;
        let double_min: f64 = f64::MIN;
        let double_inf: f64 = f64::NEG_INFINITY;
        let double_nan: f64 = f64::NAN;

        let mag_float_min: f32 = float_min.magnitude();
        let mag_float_inf: f32 = float_inf.magnitude();
        let mag_float_nan: f32 = float_nan.magnitude();
        let mag_double_min: f64 = double_min.magnitude();
        let mag_double_inf: f64 = double_inf.magnitude();
        let mag_double_nan: f64 = double_nan.magnitude();

        let mut result = String::new();

        if (mag_float_min != f32::MAX)
            || (mag_float_inf != f32::INFINITY)
            || (!mag_float_nan.is_nan())
        {
            result += "f32 failed. ";
        }
        if (mag_double_min != f64::MAX)
            || (mag_double_inf != f64::INFINITY)
            || (!mag_double_nan.is_nan())
        {
            result += "f64 failed. ";
        }

        if !result.is_empty() {
            Err(result)
        } else {
            Ok(())
        }
    }

    #[test]
    fn big() -> Result<(), String> {
        let digits: Vec<u32> = vec![9; 5];
        let unsigned: BigUint = BigUint::new(digits.clone());
        let signed: BigInt = BigInt::new(Sign::Minus, digits);
        let uzero: BigUint = BigUint::zero();
        let izero: BigInt = BigInt::zero();

        let mag_unsigned: BigUint = unsigned.clone().magnitude();
        let mag_signed: BigUint = signed.magnitude();
        let mag_uzero: BigUint = uzero.magnitude();
        let mag_izero: BigUint = izero.magnitude();

        let mut result = String::new();

        if (mag_unsigned != unsigned.clone()) || (mag_uzero != BigUint::zero()) {
            result += "BigUint failed. ";
        }
        if (mag_signed != unsigned) || (mag_izero != BigUint::zero()) {
            result += "BigInt failed. ";
        }

        if !result.is_empty() {
            Err(result)
        } else {
            Ok(())
        }
    }
}
