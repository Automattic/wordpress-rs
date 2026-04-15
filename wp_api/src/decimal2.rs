use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, Unexpected},
};
use std::fmt;

uniffi::custom_type!(Decimal2, i64, {
    lower: |decimal| decimal.hundredths(),
    try_lift: |hundredths| Ok(Decimal2::from_hundredths(hundredths)),
});

/// A decimal number with at most 2 fractional digits, stored as an integer
/// count of hundredths to avoid floating-point precision issues.
///
/// Deserialization rejects values with more than 2 decimal places rather
/// than silently rounding.
///
/// ## Examples
///
/// - `11` → `Decimal2 { hundredths: 1100 }`
/// - `8.63` → `Decimal2 { hundredths: 863 }`
/// - `8.635` → deserialization error
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Decimal2 {
    hundredths: i64,
}

impl Decimal2 {
    /// Creates a `Decimal2` from a count of hundredths (e.g. `863` → `8.63`).
    pub fn from_hundredths(hundredths: i64) -> Self {
        Self { hundredths }
    }

    /// Returns the value expressed in hundredths (e.g. `8.63` → `863`).
    pub fn hundredths(&self) -> i64 {
        self.hundredths
    }

    /// Returns the whole part of the value (e.g. `8.63` → `8`, `-3.25` → `-3`).
    pub fn whole_part(&self) -> i64 {
        self.hundredths / 100
    }

    /// Returns the fractional part as a non-negative count of hundredths
    /// (e.g. `8.63` → `63`, `-3.25` → `25`).
    pub fn fractional_hundredths(&self) -> i64 {
        (self.hundredths % 100).abs()
    }
}

impl fmt::Display for Decimal2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let whole = self.whole_part();
        let frac = self.fractional_hundredths();
        if self.hundredths < 0 && whole == 0 {
            write!(f, "-0.{frac:02}")
        } else {
            write!(f, "{whole}.{frac:02}")
        }
    }
}

impl Serialize for Decimal2 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let whole = self.whole_part();
        let frac = self.fractional_hundredths();
        if frac == 0 {
            serializer.serialize_i64(whole)
        } else {
            // Reconstruct the f64 for JSON numeric output.
            // Safe because values with at most 2 decimal places are exactly
            // representable as f64 for any realistic magnitude.
            let value = self.hundredths as f64 / 100.0;
            serializer.serialize_f64(value)
        }
    }
}

impl<'de> Deserialize<'de> for Decimal2 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(Decimal2Visitor)
    }
}

struct Decimal2Visitor;

impl de::Visitor<'_> for Decimal2Visitor {
    type Value = Decimal2;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a number with at most 2 decimal places")
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        let hundredths =
            i64::try_from(v).map_err(|_| E::invalid_value(Unexpected::Unsigned(v), &self))?;
        Ok(Decimal2 {
            hundredths: hundredths * 100,
        })
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
        Ok(Decimal2 {
            hundredths: v * 100,
        })
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
        let scaled = v * 100.0;
        let rounded = scaled.round();
        if (scaled - rounded).abs() > 1e-6 {
            return Err(E::invalid_value(
                Unexpected::Float(v),
                &"a number with at most 2 decimal places",
            ));
        }
        Ok(Decimal2 {
            hundredths: rounded as i64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct Wrapper {
        value: Decimal2,
    }

    #[rstest]
    #[case(r#"{"value": 11}"#, 1100)]
    #[case(r#"{"value": 0}"#, 0)]
    #[case(r#"{"value": 8.63}"#, 863)]
    #[case(r#"{"value": 18.0}"#, 1800)]
    #[case(r#"{"value": 1.5}"#, 150)]
    #[case(r#"{"value": -3.25}"#, -325)]
    #[case(r#"{"value": 0.01}"#, 1)]
    fn test_deserialize(#[case] json: &str, #[case] expected_hundredths: i64) {
        let wrapper: Wrapper = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(wrapper.value.hundredths(), expected_hundredths);
    }

    #[rstest]
    #[case(r#"{"value": 8.635}"#)]
    #[case(r#"{"value": 1.999}"#)]
    #[case(r#"{"value": 0.001}"#)]
    fn test_deserialize_rejects_more_than_two_decimals(#[case] json: &str) {
        let result: Result<Wrapper, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "should reject value with more than 2 decimal places"
        );
    }

    #[rstest]
    #[case(1100, "11.00")]
    #[case(863, "8.63")]
    #[case(0, "0.00")]
    #[case(-325, "-3.25")]
    #[case(-5, "-0.05")]
    fn test_display(#[case] hundredths: i64, #[case] expected: &str) {
        let d = Decimal2 { hundredths };
        assert_eq!(d.to_string(), expected);
    }

    #[rstest]
    #[case(1100, "1100")]
    #[case(863, "8.63")]
    #[case(0, "0")]
    #[case(1800, "1800")]
    #[case(150, "1.5")]
    fn test_serialize_roundtrip(#[case] hundredths: i64, #[case] _label: &str) {
        let original = Decimal2 { hundredths };
        let json = serde_json::to_string(&original).expect("should serialize");
        let restored: Decimal2 = serde_json::from_str(&json).expect("should deserialize back");
        assert_eq!(original, restored);
    }
}
