#[allow(dead_code)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct BigNumber {
    /// Significand with up to 9 digits. For `power_3 >= 1`, display is
    /// `amount / 1_000` with 3 digits after the decimal.
    amount: i64,
    power_3: usize,
}

const UNIT: i64 = 1_000;
const DISPLAY_SCALE: i64 = 1_000_000; // 3 digits after the decimal
const STORE_MAX: i64 = 1_000_000_000; // 9-digit significand boundary

#[allow(dead_code)]
impl BigNumber {
    fn new(amount: i64, power_3: usize) -> Self {
        BigNumber { amount, power_3 }.reduce()
    }

    /// Exponent of 1000 in `value = amount * 1000^scale`.
    /// Plain ints (`power_3 == 0`) and millions (`power_3 == 1`) share scale 0.
    fn scale(self) -> usize {
        self.power_3.saturating_sub(1)
    }

    fn reduce(mut self) -> Self {
        // Promote plain integers into abbreviated form at 1e6.
        if self.power_3 == 0 && self.amount.abs() >= DISPLAY_SCALE {
            self.power_3 = 1;
        }

        while self.amount.abs() >= STORE_MAX {
            self.amount /= UNIT;
            self.power_3 += 1;
        }

        while self.amount.abs() < DISPLAY_SCALE && self.power_3 != 0 {
            if self.power_3 == 1 {
                // Demote back to a plain integer.
                self.power_3 = 0;
                break;
            }
            self.amount *= UNIT;
            self.power_3 -= 1;
        }

        self
    }

    fn to_string(&self) -> String {
        if self.power_3 == 0 {
            return format!("{}", self.amount);
        }
        // power_3 >= 1: show top 3 fractional digits of the 9-digit significand
        let powers = ['M', 'B', 'T', 'Q', 'q', 's', 'S', 'O', 'N', 'D'];
        let sign = if self.amount < 0 { "-" } else { "" };
        let abs = self.amount.abs();
        format!(
            "{}{}.{:03} {}",
            sign,
            abs / DISPLAY_SCALE,
            abs % DISPLAY_SCALE / 1_000,
            powers[self.power_3 - 1]
        )
    }
}

impl From<i64> for BigNumber {
    fn from(value: i64) -> Self {
        BigNumber::new(value, 0)
    }
}

impl From<i32> for BigNumber {
    fn from(value: i32) -> Self {
        BigNumber::new(value.into(), 0)
    }
}

use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

impl Add for BigNumber {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        if self.scale() < other.scale() {
            return other + self;
        }

        let diff = self.scale() - other.scale();
        if diff >= 2 {
            self
        } else if diff == 1 {
            BigNumber {
                amount: self.amount + other.amount / UNIT,
                power_3: self.power_3.max(1),
            }
            .reduce()
        } else {
            BigNumber {
                amount: self.amount + other.amount,
                power_3: self.power_3.max(other.power_3),
            }
            .reduce()
        }
    }
}

impl AddAssign for BigNumber {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl Sub for BigNumber {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        self + BigNumber {
            amount: -other.amount,
            power_3: other.power_3,
        }
    }
}

impl SubAssign for BigNumber {
    fn sub_assign(&mut self, other: Self) {
        *self = *self - other;
    }
}

impl Mul<i32> for BigNumber {
    type Output = Self;

    fn mul(self, rhs: i32) -> Self {
        BigNumber {
            amount: self.amount * i64::from(rhs),
            power_3: self.power_3,
        }
        .reduce()
    }
}

impl MulAssign<i32> for BigNumber {
    fn mul_assign(&mut self, rhs: i32) {
        *self = *self * rhs;
    }
}

impl Div<i32> for BigNumber {
    type Output = Self;

    fn div(self, rhs: i32) -> Self {
        let div_pow = rhs.ilog10() / 3;
        let under_million = rhs / 10_i32.pow(div_pow * 3);
        BigNumber {
            amount: self.amount / i64::from(under_million),
            power_3: self.power_3 - div_pow as usize,
        }
        .reduce()
    }
}

impl DivAssign<i32> for BigNumber {
    fn div_assign(&mut self, rhs: i32) {
        *self = *self / rhs;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_zero_formats_as_plain_integer() {
        assert_eq!(BigNumber::new(0, 0).to_string(), "0");
        assert_eq!(BigNumber::new(42, 0).to_string(), "42");
        assert_eq!(BigNumber::new(-7, 0).to_string(), "-7");
        assert_eq!(BigNumber::new(999_999, 0).to_string(), "999999");
    }

    #[test]
    fn large_power_zero_reduces_into_abbreviated_form() {
        assert_eq!(BigNumber::new(1_000_000, 0).to_string(), "1.000 M");
        assert_eq!(BigNumber::new(1_234_567, 0).to_string(), "1.234 M");
        assert_eq!(BigNumber::new(999_999_999, 0).to_string(), "999.999 M");
        assert_eq!(BigNumber::new(1_500_000_000, 0).to_string(), "1.500 B");
    }

    #[test]
    fn millions_and_billions_format_with_six_decimals() {
        assert_eq!(BigNumber::new(5_000_000, 1).to_string(), "5.000 M");
        assert_eq!(BigNumber::new(1_234_567, 1).to_string(), "1.234 M");
        assert_eq!(BigNumber::new(5_000_000, 2).to_string(), "5.000 B");
        assert_eq!(BigNumber::new(1_234_567, 3).to_string(), "1.234 T");
        assert_eq!(BigNumber::new(1_234_567, 4).to_string(), "1.234 Q");
    }

    #[test]
    fn stores_nine_digits_but_displays_six_fractional() {
        // 9-digit significand 123456789 → 123.456
        assert_eq!(BigNumber::new(123_456_789, 1).to_string(), "123.456 M");
        // Still only 3 digits after the decimal in the rendered string
        let rendered = BigNumber::new(123_456_789, 1).to_string();
        let frac = rendered.split('.').nth(1).unwrap();
        assert_eq!(&frac[..3], "456");
    }

    #[test]
    fn negative_amounts_keep_sign_when_abbreviated() {
        assert_eq!(BigNumber::new(-1_500_000, 0).to_string(), "-1.500 M");
        assert_eq!(BigNumber::new(-2_500_000, 2).to_string(), "-2.500 B");
    }

    #[test]
    fn reduce_keeps_amount_in_canonical_range() {
        assert_eq!(
            BigNumber {
                amount: 1_000_000,
                power_3: 0
            }
            .reduce(),
            BigNumber::new(1_000_000, 1)
        );
        assert_eq!(
            BigNumber {
                amount: 1_000_000_000,
                power_3: 1
            }
            .reduce(),
            BigNumber::new(1_000_000, 2)
        );
        assert_eq!(
            BigNumber {
                amount: 500_000,
                power_3: 2
            }
            .reduce(),
            BigNumber::new(500_000_000, 1)
        );
        // Boundary: abs(amount) == 1e6 stays put (no infinite loop)
        assert_eq!(
            BigNumber {
                amount: DISPLAY_SCALE,
                power_3: 1
            }
            .reduce(),
            BigNumber::new(DISPLAY_SCALE, 1)
        );
        // Below display scale demotes to plain integer
        assert_eq!(
            BigNumber {
                amount: DISPLAY_SCALE - 1,
                power_3: 1
            }
            .reduce(),
            BigNumber::new(DISPLAY_SCALE - 1, 0)
        );
    }

    #[test]
    fn add_same_power() {
        let a = BigNumber::new(1_500_000, 1);
        let b = BigNumber::new(2_500_000, 1);
        assert_eq!((a + b).to_string(), "4.000 M");
    }

    #[test]
    fn add_one_power_apart() {
        let millions = BigNumber::new(1_500_000, 1); // 1.500000 M
        let billions = BigNumber::new(2_000_000, 2); // 2.000000 B
        // 2.000000 B + 0.001500 B = 2.001500 B
        assert_eq!((billions + millions).to_string(), "2.001 B");
        assert_eq!(millions + billions, billions + millions);
    }

    #[test]
    fn add_two_or_more_powers_apart_keeps_larger() {
        let millions = BigNumber::new(999_999_999, 1);
        let trillions = BigNumber::new(1_000_000, 3);
        assert_eq!(trillions + millions, trillions);
        assert_eq!(millions + trillions, trillions);
    }

    #[test]
    fn add_can_carry_into_next_power() {
        let a = BigNumber::from(600_000_000);
        let b = BigNumber::from(600_000_000);
        assert_eq!((a + b).to_string(), "1.200 B");
    }

    #[test]
    fn add_assign_matches_add() {
        let mut n = BigNumber::new(1_500_000, 1);
        n += BigNumber::new(500_000, 0); // plain 500_000 shares scale with millions
        assert_eq!(n, BigNumber::new(2_000_000, 1));
    }

    #[test]
    fn sub_same_power() {
        let a = BigNumber::new(5_000_000, 1);
        let b = BigNumber::new(1_500_000, 1);
        assert_eq!((a - b).to_string(), "3.500 M");
    }

    #[test]
    fn sub_assign_matches_sub() {
        let mut n = BigNumber::new(5_000_000, 1);
        n -= BigNumber::new(2_000_000, 1);
        assert_eq!(n, BigNumber::new(3_000_000, 1));
    }

    #[test]
    fn mul_by_int_and_reduce() {
        let n = BigNumber::new(1_500_000, 1); // 1.500000 M
        assert_eq!((n * 2).to_string(), "3.000 M");
        assert_eq!((n * 1000).to_string(), "1.500 B");
    }

    #[test]
    fn mul_assign_matches_mul() {
        let mut n = BigNumber::new(1_500_000, 1);
        n *= 3;
        assert_eq!(n, BigNumber::new(4_500_000, 1));
    }

    #[test]
    fn div_by_small_int() {
        let n = BigNumber::new(6_000_000, 1); // 6.000000 M
        assert_eq!((n / 2).to_string(), "3.000 M");
    }

    #[test]
    fn div_by_thousand_drops_power() {
        let n = BigNumber::new(5_000_000, 2); // 5.000000 B
        assert_eq!((n / 1000).to_string(), "5.000 M");
    }

    #[test]
    fn div_assign_matches_div() {
        let mut n = BigNumber::new(8_000_000, 1);
        n /= 4;
        assert_eq!(n, BigNumber::new(2_000_000, 1));
    }
}
