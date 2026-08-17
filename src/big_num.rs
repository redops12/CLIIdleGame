use std::cmp::Ordering;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

use serde::{Deserialize, Serialize};

const MILLI: f64 = 1_000.0;
const POWERS: [char; 10] = ['M', 'B', 'T', 'Q', 'q', 's', 'S', 'O', 'N', 'D'];

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BigDollar(f64);

impl BigDollar {
    fn format_plain_milli(milli: i64) -> String {
        if milli == 0 {
            return "0".to_string();
        }
        let sign = if milli < 0 { "-" } else { "" };
        let abs = milli.abs();
        let whole = abs / 1_000;
        let frac = abs % 1_000;
        if frac == 0 {
            format!("{}{}", sign, whole)
        } else if whole == 0 {
            format!("{}0.{:03}", sign, frac)
        } else {
            format!("{}{}.{:03}", sign, whole, frac)
        }
    }

    fn abbrev_power_3(abs_milli: i64) -> usize {
        debug_assert!(abs_milli >= 1_000_000_000);
        let mut power = 1usize;
        let mut threshold: i64 = 1_000_000_000_000;
        while abs_milli >= threshold && power < POWERS.len() {
            power += 1;
            threshold = threshold.saturating_mul(1000);
        }
        power
    }

    fn format_abbreviated(sign: &str, abs_milli: i64) -> String {
        let power_3 = Self::abbrev_power_3(abs_milli);
        // Match the old significand layout: truncate (don't round) to 3 displayed decimals.
        let amount = abs_milli / 1000_i64.pow(power_3 as u32);
        format!(
            "{}{}.{:03} {}",
            sign,
            amount / 1_000_000,
            (amount % 1_000_000) / 1_000,
            POWERS[power_3 - 1]
        )
    }

    fn format_amount(&self) -> String {
        let value = self.0;
        if value == 0.0 {
            return "0".to_string();
        }

        let sign = if value < 0.0 { "-" } else { "" };
        let abs_milli = (value.abs() * MILLI).round() as i64;
        if abs_milli == 0 {
            return "0".to_string();
        }

        // Plain values: round to the nearest milli-unit first, then format.
        const ABBREV_MILLI: i64 = 1_000_000_000; // 1M display units
        if abs_milli < ABBREV_MILLI {
            return format!("{sign}{}", Self::format_plain_milli(abs_milli));
        }

        Self::format_abbreviated(sign, abs_milli)
    }
}

impl From<i64> for BigDollar {
    fn from(value: i64) -> Self {
        BigDollar(value as f64 / MILLI)
    }
}

impl From<i32> for BigDollar {
    fn from(value: i32) -> Self {
        BigDollar::from(i64::from(value))
    }
}

impl From<f64> for BigDollar {
    fn from(value: f64) -> Self {
        BigDollar(value)
    }
}

impl std::fmt::Display for BigDollar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let body = self.format_amount();
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
        BigDollar(self.0 * f64::from(rhs))
    }
}

impl Mul<f32> for BigDollar {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self {
        BigDollar(self.0 * f64::from(rhs))
    }
}

impl Mul<f64> for BigDollar {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self {
        BigDollar(self.0 * rhs)
    }
}

impl MulAssign<i32> for BigDollar {
    fn mul_assign(&mut self, rhs: i32) {
        self.0 *= f64::from(rhs);
    }
}

impl MulAssign<f32> for BigDollar {
    fn mul_assign(&mut self, rhs: f32) {
        self.0 *= f64::from(rhs);
    }
}

impl MulAssign<f64> for BigDollar {
    fn mul_assign(&mut self, rhs: f64) {
        self.0 *= rhs;
    }
}

impl Div<i32> for BigDollar {
    type Output = Self;

    fn div(self, rhs: i32) -> Self {
        BigDollar(self.0 / f64::from(rhs))
    }
}

impl DivAssign<i32> for BigDollar {
    fn div_assign(&mut self, rhs: i32) {
        self.0 /= f64::from(rhs);
    }
}

impl PartialOrd for BigDollar {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_int_uses_milli_units() {
        assert_eq!(BigDollar::from(0).format_amount(), "0");
        assert_eq!(BigDollar::from(1).format_amount(), "0.001");
        assert_eq!(BigDollar::from(42).format_amount(), "0.042");
        assert_eq!(BigDollar::from(1000).format_amount(), "1");
        assert_eq!(BigDollar::from(1500).format_amount(), "1.500");
        assert_eq!(BigDollar::from(-7).format_amount(), "-0.007");
        assert_eq!(BigDollar::from(1).to_string(), "$0.001");
        assert_eq!(BigDollar::from(1000).to_string(), "$1");
    }

    #[test]
    fn plain_values_format_with_milli_precision() {
        assert_eq!(BigDollar::from(0).format_amount(), "0");
        assert_eq!(BigDollar::from(42).format_amount(), "0.042");
        assert_eq!(BigDollar::from(-7).format_amount(), "-0.007");
        assert_eq!(BigDollar::from(999_999).format_amount(), "999.999");
    }

    #[test]
    fn large_values_use_abbreviated_form() {
        assert_eq!(BigDollar::from(1_000_000).format_amount(), "1000");
        assert_eq!(BigDollar::from(1_234_567).format_amount(), "1234.567");
        assert_eq!(BigDollar::from(999_999_999).format_amount(), "999999.999");
        assert_eq!(BigDollar::from(1_000_000_000_i64).format_amount(), "1.000 M");
        assert_eq!(BigDollar::from(1_234_567_000_i64).format_amount(), "1.234 M");
        assert_eq!(BigDollar::from(1_500_000_000_i64).format_amount(), "1.500 M");
    }

    #[test]
    fn millions_and_billions_format_with_three_decimals() {
        assert_eq!(BigDollar::from(5_000_000_000_i64).format_amount(), "5.000 M");
        assert_eq!(BigDollar::from(1_234_567_000_i64).format_amount(), "1.234 M");
        assert_eq!(BigDollar::from(5_000_000_000_000_i64).format_amount(), "5.000 B");
        assert_eq!(BigDollar::from(1_234_567_000_000_000_i64).format_amount(), "1.234 T");
        assert_eq!(BigDollar::from(1_234_567_000_000_000_000_i64).format_amount(), "1.234 Q");
    }

    #[test]
    fn abbreviated_values_show_three_fractional_digits() {
        assert_eq!(BigDollar::from(123_456_789_000_i64).format_amount(), "123.456 M");
        let rendered = BigDollar::from(123_456_789_000_i64).format_amount();
        let frac = rendered.split('.').nth(1).unwrap();
        assert_eq!(&frac[..3], "456");
    }

    #[test]
    fn negative_values_keep_sign_when_abbreviated() {
        assert_eq!(BigDollar::from(-1_500_000_000_i64).format_amount(), "-1.500 M");
        assert_eq!(BigDollar::from(-2_500_000_000_000_i64).format_amount(), "-2.500 B");
    }

    #[test]
    fn from_f64_uses_display_units() {
        assert_eq!(BigDollar::from(1.5).format_amount(), "1.500");
        assert_eq!(BigDollar::from(1_500_000.0).format_amount(), "1.500 M");
    }

    #[test]
    fn add_same_magnitude() {
        let a = BigDollar::from(1_500_000_000_i64);
        let b = BigDollar::from(2_500_000_000_i64);
        assert_eq!((a + b).format_amount(), "4.000 M");
    }

    #[test]
    fn add_different_magnitudes() {
        let millions = BigDollar::from(1_500_000_000_i64);
        let billions = BigDollar::from(2_000_000_000_000_i64);
        assert_eq!((billions + millions).format_amount(), "2.001 B");
        assert_eq!(millions + billions, billions + millions);

        let near_trillion = BigDollar::from(999_999_999_000_i64);
        let trillion = BigDollar::from(1_000_000_000_000_000_i64);
        assert_eq!(trillion + near_trillion, BigDollar::from(1_000_999_999_999_000_i64));
    }

    #[test]
    fn add_can_carry_into_next_power() {
        let a = BigDollar::from(600_000_000_000_i64);
        let b = BigDollar::from(600_000_000_000_i64);
        assert_eq!((a + b).format_amount(), "1.200 B");
    }

    #[test]
    fn add_assign_matches_add() {
        let mut money = BigDollar::from(1_500_000_000_i64);
        money += 500_000_000_i64;
        assert_eq!(money, BigDollar::from(2_000_000_000_i64));
    }

    #[test]
    fn sub_same_magnitude() {
        let a = BigDollar::from(5_000_000_000_i64);
        let b = BigDollar::from(1_500_000_000_i64);
        assert_eq!((a - b).format_amount(), "3.500 M");
    }

    #[test]
    fn sub_assign_matches_sub() {
        let mut money = BigDollar::from(5_000_000_000_i64);
        money -= BigDollar::from(2_000_000_000_i64);
        assert_eq!(money, BigDollar::from(3_000_000_000_i64));
    }

    #[test]
    fn mul_by_int() {
        let money = BigDollar::from(1_500_000_000_i64);
        assert_eq!((money * 2).format_amount(), "3.000 M");
        assert_eq!((money * 1000).format_amount(), "1.500 B");
    }

    #[test]
    fn mul_by_float() {
        let money = BigDollar::from(1.0);
        assert_eq!((money * 2.0_f64).format_amount(), "2");
    }

    #[test]
    fn mul_assign_matches_mul() {
        let mut money = BigDollar::from(1_500_000_000_i64);
        money *= 3;
        assert_eq!(money, BigDollar::from(4_500_000_000_i64));
    }

    #[test]
    fn div_by_small_int() {
        let money = BigDollar::from(6_000_000_000_i64);
        assert_eq!((money / 2).format_amount(), "3.000 M");
    }

    #[test]
    fn div_by_thousand_drops_power() {
        let money = BigDollar::from(5_000_000_000_000_i64);
        assert_eq!((money / 1000).format_amount(), "5.000 M");
    }

    #[test]
    fn div_assign_matches_div() {
        let mut money = BigDollar::from(8_000_000_000_i64);
        money /= 4;
        assert_eq!(money, BigDollar::from(2_000_000_000_i64));
    }

    #[test]
    fn display_prefixes_dollar_sign() {
        assert_eq!(BigDollar::from(0).to_string(), "$0");
        assert_eq!(BigDollar::from(42_i32).to_string(), "$0.042");
        assert_eq!(BigDollar::from(1_000_000_000_i64).to_string(), "$1.000 M");
        assert_eq!(BigDollar::from(-7_i32).to_string(), "-$0.007");
    }

    #[test]
    fn ops_with_int_literals() {
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
    fn cmp_same_scale() {
        assert!(BigDollar::from(10) < BigDollar::from(20));
        assert!(BigDollar::from(20) > BigDollar::from(10));
        assert!(BigDollar::from(10) <= BigDollar::from(10));
        assert!(BigDollar::from(10) >= BigDollar::from(10));
        assert_eq!(BigDollar::from(10), BigDollar::from(10));
        assert!(BigDollar::from(-5) < BigDollar::from(3));
        assert!(BigDollar::from(-10) < BigDollar::from(-3));
    }

    #[test]
    fn cmp_zero_against_nonzero() {
        assert!(BigDollar::from(0) < BigDollar::from(100));
        assert!(BigDollar::from(0) < BigDollar::from(1));
        assert!(!(BigDollar::from(0) >= BigDollar::from(100)));
        assert!(BigDollar::from(0) > BigDollar::from(-1));
    }

    #[test]
    fn cmp_across_scales() {
        let millions = BigDollar::from(1_500_000_000_i64);
        let billions = BigDollar::from(2_000_000_000_000_i64);
        let trillions = BigDollar::from(1_000_000_000_000_000_i64);

        assert!(millions < billions);
        assert!(billions < trillions);
        assert!(millions < trillions);
        assert!(BigDollar::from(-1_000_000_000_000_000_i64) < BigDollar::from(-1_500_000_000_i64));
        assert!(BigDollar::from(999_999_999) < millions);
    }

    #[test]
    fn display_rounds_f64_noise_in_plain_values() {
        assert_eq!(BigDollar(0.1 + 0.2).format_amount(), "0.300");
        assert_eq!(BigDollar(1.0 / 3.0 * 3.0).format_amount(), "1");

        let mut sum = BigDollar(0.0);
        for _ in 0..1_000 {
            sum += BigDollar::from(1); // +0.001 per iteration
        }
        assert_eq!(sum.format_amount(), "1");
    }

    #[test]
    fn display_rounds_upgrade_cost_powers() {
        for i in 0..=49 {
            let cost = BigDollar(50.0 * 1.1_f64.powi(i));
            let rendered = cost.format_amount();
            assert!(
                rendered.contains('.') || rendered.parse::<i64>().is_ok(),
                "upgrade cost level {i} was not a plain number: {rendered}"
            );
        }
        assert_eq!(BigDollar(50.0 * 1.1_f64.powi(5)).format_amount(), "80.526");
        assert_eq!(BigDollar(50.0 * 1.1_f64.powi(49)).format_amount(), "5335.948");
    }

    #[test]
    fn display_rounds_trust_multiplier_values() {
        let base = BigDollar::from(1);
        assert_eq!((base * 2.0_f64.powi(10)).format_amount(), "1.024");
        assert_eq!((base * 2.0_f64.powi(20)).format_amount(), "1048.576");
    }

    #[test]
    fn display_switches_to_abbreviated_after_milli_rounding() {
        assert_eq!(BigDollar(999_999.9995).format_amount(), "1.000 M");
        assert_eq!(BigDollar(999_999.9994).format_amount(), "999999.999");
        assert_eq!(BigDollar(1_000_000.0).format_amount(), "1.000 M");
    }

    #[test]
    fn display_rounds_abbreviated_suffix_boundaries() {
        assert_eq!(BigDollar::from(999_999_000_000_i64).format_amount(), "999.999 M");
        assert_eq!(BigDollar(999_999_999.9995).format_amount(), "1.000 B");
        assert_eq!(BigDollar::from(1_000_000_000_000_i64).format_amount(), "1.000 B");
        assert_eq!(BigDollar(1_000_000_000.0).format_amount(), "1.000 B");
        assert_eq!(BigDollar::from(999_999_999_999_499_i64).format_amount(), "999.999 B");
        assert_eq!(BigDollar(999_999_999_999.9995).format_amount(), "1.000 T");
        assert_eq!(BigDollar(1_000_000_000_000.0).format_amount(), "1.000 T");
    }

    #[test]
    fn display_rounds_division_remainders() {
        assert_eq!((BigDollar(1500.0) / 3).format_amount(), "500");
        assert_eq!((BigDollar(1000.0) / 3).format_amount(), "333.333");
    }
}
