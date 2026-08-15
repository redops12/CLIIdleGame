#[allow(dead_code)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BigNumber {
    /// Significand with up to 9 digits. For `power_3 >= 1`, display is
    /// `amount / 1_000` with 3 digits after the decimal.
    amount: i64,
    power_3: usize,
}

const UNIT: i64 = 1_000;
/// Integer literals in `From<i64>` are milli-units: `from(1)` = 0.001 display units.
const MILLI: i64 = 1_000;
const DISPLAY_SCALE: i64 = 1_000_000; // significand → abbreviated display (3 fractional digits)
const PROMOTE_AT: i64 = 1_000_000_000; // 1M display units in milli-units
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
        // Promote milli-units into abbreviated significand form at 1M display units.
        if self.power_3 == 0 && self.amount.abs() >= PROMOTE_AT {
            self.amount /= MILLI;
            self.power_3 = 1;
        }

        while self.amount.abs() >= STORE_MAX {
            self.amount /= UNIT;
            self.power_3 += 1;
        }

        while self.amount.abs() < DISPLAY_SCALE && self.power_3 != 0 {
            if self.power_3 == 1 {
                // Demote back to milli-units.
                self.amount *= MILLI;
                self.power_3 = 0;
                break;
            }
            self.amount *= UNIT;
            self.power_3 -= 1;
        }

        self
    }

    fn format_plain_milli(amount: i64) -> String {
        if amount == 0 {
            return "0".to_string();
        }
        let sign = if amount < 0 { "-" } else { "" };
        let abs = amount.abs();
        let whole = abs / MILLI;
        let frac = abs % MILLI;
        if frac == 0 {
            format!("{}{}", sign, whole)
        } else if whole == 0 {
            format!("{}0.{:03}", sign, frac)
        } else {
            format!("{}{}.{:03}", sign, whole, frac)
        }
    }

    fn to_milli(self) -> i64 {
        if self.power_3 == 0 {
            self.amount
        } else {
            self.amount.saturating_mul(MILLI)
        }
    }

    fn to_string(&self) -> String {
        if self.power_3 == 0 {
            return Self::format_plain_milli(self.amount);
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

impl From<f64> for BigNumber {
    fn from(value: f64) -> Self {
        BigNumber::new(value.round() as i64, 0)
    }
}

use std::cmp::Ordering;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

impl PartialOrd for BigNumber {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BigNumber {
    fn cmp(&self, other: &Self) -> Ordering {
        // negative < zero < positive
        let self_sign = self.amount.cmp(&0);
        let other_sign = other.amount.cmp(&0);
        if self_sign != other_sign {
            return self_sign.cmp(&other_sign);
        }
        if self.amount == 0 {
            return Ordering::Equal;
        }

        let self_milli = self.to_milli().abs();
        let other_milli = other.to_milli().abs();
        let magnitude = match self.scale().cmp(&other.scale()) {
            Ordering::Greater => {
                let diff = self.scale() - other.scale();
                if diff >= 2 {
                    Ordering::Greater
                } else {
                    (self_milli * UNIT).cmp(&other_milli)
                }
            }
            Ordering::Less => {
                let diff = other.scale() - self.scale();
                if diff >= 2 {
                    Ordering::Less
                } else {
                    self_milli.cmp(&(other_milli * UNIT))
                }
            }
            Ordering::Equal => self_milli.cmp(&other_milli),
        };

        if self.amount < 0 {
            magnitude.reverse()
        } else {
            magnitude
        }
    }
}

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
            let other_contribution = if other.power_3 == 0 {
                other.amount / MILLI
            } else {
                other.amount / UNIT
            };
            BigNumber {
                amount: self.amount + other_contribution,
                power_3: self.power_3.max(1),
            }
            .reduce()
        } else {
            let (amount, power_3) = match (self.power_3, other.power_3) {
                (0, 0) => (self.amount + other.amount, 0),
                (0, _) => (self.amount / MILLI + other.amount, other.power_3),
                (_, 0) => (self.amount + other.amount / MILLI, self.power_3),
                (_, _) => (
                    self.amount + other.amount,
                    self.power_3.max(other.power_3),
                ),
            };
            BigNumber { amount, power_3 }.reduce()
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

impl Mul<f32> for BigNumber {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self {
        BigNumber {
            amount: ((self.amount as f32) * rhs).round() as i64,
            power_3: self.power_3,
        }
        .reduce()
    }
}

impl MulAssign<f32> for BigNumber {
    fn mul_assign(&mut self, rhs: f32) {
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BigDollar(BigNumber);

impl std::ops::Deref for BigDollar {
    type Target = BigNumber;
    fn deref(&self) -> &BigNumber {
        &self.0
    }
}

impl std::ops::DerefMut for BigDollar {
    fn deref_mut(&mut self) -> &mut BigNumber {
        &mut self.0
    }
}

impl From<BigNumber> for BigDollar {
    fn from(value: BigNumber) -> Self {
        BigDollar(value)
    }
}

impl From<i64> for BigDollar {
    fn from(value: i64) -> Self {
        BigDollar(BigNumber::from(value))
    }
}

impl From<i32> for BigDollar {
    fn from(value: i32) -> Self {
        BigDollar(BigNumber::from(value))
    }
}

impl From<f64> for BigDollar {
    fn from(value: f64) -> Self {
        BigDollar(BigNumber::from(value))
    }
}

impl std::fmt::Display for BigDollar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let body = self.0.to_string();
        if let Some(rest) = body.strip_prefix('-') {
            write!(f, "-${rest}")
        } else {
            write!(f, "${body}")
        }
    }
}

impl Add for BigDollar {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        BigDollar(self.0 + other.0)
    }
}

impl Add<i64> for BigDollar {
    type Output = Self;

    fn add(self, other: i64) -> Self {
        self + BigDollar::from(other)
    }
}

impl Add<i32> for BigDollar {
    type Output = Self;

    fn add(self, other: i32) -> Self {
        self + BigDollar::from(other)
    }
}

impl AddAssign for BigDollar {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl AddAssign<i64> for BigDollar {
    fn add_assign(&mut self, other: i64) {
        *self += BigDollar::from(other);
    }
}

impl AddAssign<i32> for BigDollar {
    fn add_assign(&mut self, other: i32) {
        *self += BigDollar::from(other);
    }
}

impl Sub for BigDollar {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        BigDollar(self.0 - other.0)
    }
}

impl Sub<i64> for BigDollar {
    type Output = Self;

    fn sub(self, other: i64) -> Self {
        self - BigDollar::from(other)
    }
}

impl Sub<i32> for BigDollar {
    type Output = Self;

    fn sub(self, other: i32) -> Self {
        self - BigDollar::from(other)
    }
}

impl SubAssign for BigDollar {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

impl SubAssign<i64> for BigDollar {
    fn sub_assign(&mut self, other: i64) {
        *self -= BigDollar::from(other);
    }
}

impl SubAssign<i32> for BigDollar {
    fn sub_assign(&mut self, other: i32) {
        *self -= BigDollar::from(other);
    }
}

impl Mul<i32> for BigDollar {
    type Output = Self;

    fn mul(self, rhs: i32) -> Self {
        BigDollar(self.0 * rhs)
    }
}

impl MulAssign<i32> for BigDollar {
    fn mul_assign(&mut self, rhs: i32) {
        self.0 *= rhs;
    }
}

impl Mul<f32> for BigDollar {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self {
        BigDollar(self.0 * rhs)
    }
}

impl MulAssign<f32> for BigDollar {
    fn mul_assign(&mut self, rhs: f32) {
        self.0 *= rhs;
    }
}

impl Div<i32> for BigDollar {
    type Output = Self;

    fn div(self, rhs: i32) -> Self {
        BigDollar(self.0 / rhs)
    }
}

impl DivAssign<i32> for BigDollar {
    fn div_assign(&mut self, rhs: i32) {
        self.0 /= rhs;
    }
}

impl PartialOrd for BigDollar {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BigDollar {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_int_uses_milli_units() {
        assert_eq!(BigNumber::from(0).to_string(), "0");
        assert_eq!(BigNumber::from(1).to_string(), "0.001");
        assert_eq!(BigNumber::from(42).to_string(), "0.042");
        assert_eq!(BigNumber::from(1000).to_string(), "1");
        assert_eq!(BigNumber::from(1500).to_string(), "1.500");
        assert_eq!(BigNumber::from(-7).to_string(), "-0.007");
        assert_eq!(BigDollar::from(1).to_string(), "$0.001");
        assert_eq!(BigDollar::from(1000).to_string(), "$1");
    }

    #[test]
    fn power_zero_formats_as_plain_milli() {
        assert_eq!(BigNumber::new(0, 0).to_string(), "0");
        assert_eq!(BigNumber::new(42, 0).to_string(), "0.042");
        assert_eq!(BigNumber::new(-7, 0).to_string(), "-0.007");
        assert_eq!(BigNumber::new(999_999, 0).to_string(), "999.999");
    }

    #[test]
    fn large_power_zero_reduces_into_abbreviated_form() {
        assert_eq!(BigNumber::new(1_000_000, 0).to_string(), "1000");
        assert_eq!(BigNumber::new(1_234_567, 0).to_string(), "1234.567");
        assert_eq!(BigNumber::new(999_999_999, 0).to_string(), "999999.999");
        assert_eq!(BigNumber::new(1_000_000_000, 0).to_string(), "1.000 M");
        assert_eq!(BigNumber::new(1_500_000_000, 0).to_string(), "1.500 M");
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
        assert_eq!(BigNumber::new(-1_500_000_000, 0).to_string(), "-1.500 M");
        assert_eq!(BigNumber::new(-2_500_000, 2).to_string(), "-2.500 B");
    }

    #[test]
    fn reduce_keeps_amount_in_canonical_range() {
        assert_eq!(
            BigNumber {
                amount: 1_000_000_000,
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
        // Boundary: abs(amount) == 1e6 significand stays put (no infinite loop)
        assert_eq!(
            BigNumber {
                amount: DISPLAY_SCALE,
                power_3: 1
            }
            .reduce(),
            BigNumber::new(DISPLAY_SCALE, 1)
        );
        // Below display scale demotes to milli-units
        assert_eq!(
            BigNumber {
                amount: DISPLAY_SCALE - 1,
                power_3: 1
            }
            .reduce(),
            BigNumber::new((DISPLAY_SCALE - 1) * MILLI, 0)
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
        let a = BigNumber::from(600_000_000_000_i64);
        let b = BigNumber::from(600_000_000_000_i64);
        assert_eq!((a + b).to_string(), "1.200 B");
    }

    #[test]
    fn add_assign_matches_add() {
        let mut n = BigNumber::new(1_500_000, 1);
        n += BigNumber::new(500_000_000, 0); // 500_000 display units in milli-units
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

    #[test]
    fn big_dollar_from_ints_and_display() {
        assert_eq!(BigDollar::from(0).to_string(), "$0");
        assert_eq!(BigDollar::from(42_i32).to_string(), "$0.042");
        assert_eq!(BigDollar::from(1_000_000_000_i64).to_string(), "$1.000 M");
        assert_eq!(BigDollar::from(-7_i32).to_string(), "-$0.007");
    }

    #[test]
    fn big_dollar_ops_forward_to_big_number() {
        let mut money = BigDollar::from(1_000_000);
        money += 500_000_i64;
        assert_eq!(money, BigDollar::from(1_500_000));

        money -= 200_000_i32;
        assert_eq!(money, BigDollar::from(1_300_000));

        money = money + BigDollar::from(200_000);
        assert_eq!(money, BigDollar::from(1_500_000));

        money = money * 2;
        assert_eq!(money.to_string(), "$3000");

        money /= 3;
        assert_eq!(money, BigDollar::from(1_000_000));
    }

    #[test]
    fn big_number_cmp_same_scale() {
        assert!(BigNumber::from(10) < BigNumber::from(20));
        assert!(BigNumber::from(20) > BigNumber::from(10));
        assert!(BigNumber::from(10) <= BigNumber::from(10));
        assert!(BigNumber::from(10) >= BigNumber::from(10));
        assert_eq!(BigNumber::from(10), BigNumber::from(10));
        assert!(BigNumber::from(-5) < BigNumber::from(3));
        assert!(BigNumber::from(-10) < BigNumber::from(-3));
    }

    #[test]
    fn big_number_cmp_zero_against_nonzero() {
        assert!(BigNumber::from(0) < BigNumber::from(100));
        assert!(BigNumber::from(0) < BigNumber::from(1));
        assert!(!(BigNumber::from(0) >= BigNumber::from(100)));
        assert!(BigNumber::from(0) > BigNumber::from(-1));
        assert!(BigDollar::from(0) < BigDollar::from(100));
        assert!(!(BigDollar::from(0) >= BigDollar::from(100)));
    }

    #[test]
    fn big_number_cmp_across_scales() {
        let millions = BigNumber::new(1_500_000, 1);
        let billions = BigNumber::new(2_000_000, 2);
        let trillions = BigNumber::new(1_000_000, 3);

        assert!(millions < billions);
        assert!(billions < trillions);
        assert!(millions < trillions);
        assert!(BigNumber::new(-1_000_000, 3) < BigNumber::new(-1_500_000, 1));
        assert!(BigNumber::from(999_999_999) < millions);
    }

    #[test]
    fn big_dollar_cmp_matches_big_number() {
        assert!(BigDollar::from(100) < BigDollar::from(200));
        assert!(BigDollar::from(200) > BigDollar::from(100));
        assert!(BigDollar::from(100) <= BigDollar::from(100));
        assert_eq!(BigDollar::from(100), BigDollar::from(100));
        assert!(BigDollar::from(-50) < BigDollar::from(0));
    }
}
