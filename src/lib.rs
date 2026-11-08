use std::fmt::{self, Display};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

pub mod config;
pub use config::Config;

mod error;
pub use crate::error::{Error, Result};

pub mod interface;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Quantity {
    pub base: isize,
    pub magnitude: i8,
    pub power: i8,
}

impl Serialize for Quantity {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = self.to_string();
        serializer.serialize_str(&value)
    }
}

impl<'de> Deserialize<'de> for Quantity {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Quantity::parse(&s).map_err(de::Error::custom)
    }
}

impl Quantity {
    /// Parse a quantity string like "4G", "512M", "1024K"
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim();

        if s.is_empty() {
            return Err(Error::QuantityEmpty);
        }

        // Find where the numeric part ends
        let numeric_end = s
            .char_indices()
            .find(|&(_, c)| !c.is_ascii_digit() && c != '-' && c != '+')
            .map_or(s.len(), |(i, _)| i);

        if numeric_end == 0 {
            return Err(Error::InvalidQuantity(s.to_string()));
        }

        let (num_str, suffix) = s.split_at(numeric_end);
        let base: isize = num_str.parse().map_err(Error::ParseQuantity)?;

        let (magnitude, power) = match suffix.trim().to_uppercase().as_str() {
            "" | "B" => (0, 0),      // Bytes
            "K" | "KB" => (1, 3),    // Kilobytes (10^3)
            "KI" | "KIB" => (1, 10), // Kibibytes (2^10)
            "M" | "MB" => (1, 6),    // Megabytes (10^6)
            "MI" | "MIB" => (1, 20), // Mebibytes (2^20)
            "G" | "GB" => (1, 9),    // Gigabytes (10^9)
            "GI" | "GIB" => (1, 30), // Gibibytes (2^30)
            "T" | "TB" => (1, 12),   // Terabytes (10^12)
            "TI" | "TIB" => (1, 40), // Tebibytes (2^40)
            _ => return Err(Error::InvalidQuantity(s.to_string())),
        };

        Ok(Quantity {
            base,
            magnitude,
            power,
        })
    }

    /// Resolve to an absolute byte count.
    pub fn to_bytes(&self) -> Result<i64> {
        let multiplier: i64 = match self.power {
            0 => 1,
            3 => 1_000,
            6 => 1_000_000,
            9 => 1_000_000_000,
            12 => 1_000_000_000_000,
            10 => 1i64 << 10,
            20 => 1i64 << 20,
            30 => 1i64 << 30,
            40 => 1i64 << 40,
            _ => return Err(Error::InvalidQuantity(self.to_string())),
        };

        (self.base as i64)
            .checked_mul(multiplier)
            .ok_or_else(|| Error::InvalidQuantity(self.to_string()))
    }
}

impl Display for Quantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.magnitude == 0 && self.power == 0 {
            write!(f, "{}", self.base)
        } else {
            let suffix = match (self.magnitude, self.power) {
                (0, 0) => "",
                (1, 3) => "K",
                (1, 10) => "Ki",
                (1, 6) => "M",
                (1, 20) => "Mi",
                (1, 9) => "G",
                (1, 30) => "Gi",
                (1, 12) => "T",
                (1, 40) => "Ti",
                _ => return write!(f, "{}*{}^{}", self.base, self.magnitude, self.power),
            };
            write!(f, "{}{}", self.base, suffix)
        }
    }
}

/// A cgroup v2 limit expressed as a byte quantity: either a specific
/// [`Quantity`], or the literal "max" (unlimited). Used by any controller
/// whose interface files take a size — `memory.max`/`high`/`low`/`min`/
/// `swap.max`/`swap.high`/`zswap.max`, and `hugetlb.<size>.max`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QuantityLimit {
    /// Specific limit
    Value(Quantity),
    /// Unlimited
    Max,
}

impl Display for QuantityLimit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QuantityLimit::Value(q) => write!(f, "{q}"),
            QuantityLimit::Max => write!(f, "max"),
        }
    }
}

impl Serialize for QuantityLimit {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            QuantityLimit::Value(q) => q.serialize(serializer),
            QuantityLimit::Max => serializer.serialize_str("max"),
        }
    }
}

impl<'de> Deserialize<'de> for QuantityLimit {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct QuantityLimitVisitor;

        impl<'de> de::Visitor<'de> for QuantityLimitVisitor {
            type Value = QuantityLimit;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a quantity string or \"max\"")
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                if value == "max" {
                    Ok(QuantityLimit::Max)
                } else {
                    Quantity::parse(value)
                        .map(QuantityLimit::Value)
                        .map_err(de::Error::custom)
                }
            }
        }

        deserializer.deserialize_str(QuantityLimitVisitor)
    }
}

impl From<Quantity> for QuantityLimit {
    fn from(value: Quantity) -> Self {
        Self::Value(value)
    }
}

impl QuantityLimit {
    /// Render this limit for writing directly to a cgroup v2 interface
    /// file. Those files take raw byte counts (or the literal string
    /// "max"), never the human-readable suffixed form ("4G") that
    /// `Display` produces.
    pub fn to_cgroup_value(&self) -> Result<String> {
        match self {
            QuantityLimit::Max => Ok("max".to_string()),
            QuantityLimit::Value(q) => Ok(q.to_bytes()?.to_string()),
        }
    }
}

/// A cgroup v2 limit expressed as a plain count: either a specific `u64`,
/// or the literal "max" (unlimited). Used by any controller whose
/// interface files take a bare count rather than a byte quantity —
/// `pids.max`, `io.max`'s `rbps`/`wbps`/`riops`/`wiops`, and `rdma.max`'s
/// `hca_handle`/`hca_object`.
///
/// Distinct from [`QuantityLimit`] because these deserialize from a bare
/// number *or* the string "max" — never from quantity text like "4G" —
/// so a plain `u64` in config stays a plain, unquoted number.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CountLimit {
    /// Specific limit
    Value(u64),
    /// Unlimited
    Max,
}

impl Display for CountLimit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CountLimit::Value(v) => write!(f, "{v}"),
            CountLimit::Max => write!(f, "max"),
        }
    }
}

impl Serialize for CountLimit {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            CountLimit::Value(v) => serializer.serialize_u64(*v),
            CountLimit::Max => serializer.serialize_str("max"),
        }
    }
}

impl<'de> Deserialize<'de> for CountLimit {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CountLimitVisitor;

        impl<'de> de::Visitor<'de> for CountLimitVisitor {
            type Value = CountLimit;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a u64 or the string \"max\"")
            }

            fn visit_u64<E>(self, value: u64) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(CountLimit::Value(value))
            }

            fn visit_i64<E>(self, value: i64) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                u64::try_from(value)
                    .map(CountLimit::Value)
                    .map_err(|_| de::Error::invalid_value(de::Unexpected::Signed(value), &self))
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                if value == "max" {
                    Ok(CountLimit::Max)
                } else {
                    Err(de::Error::unknown_variant(value, &["max"]))
                }
            }
        }

        deserializer.deserialize_any(CountLimitVisitor)
    }
}

impl From<u64> for CountLimit {
    fn from(value: u64) -> Self {
        Self::Value(value)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn quantity_parse() {
        // Test basic parsing
        assert_eq!(
            Quantity::parse("100").unwrap(),
            Quantity {
                base: 100,
                magnitude: 0,
                power: 0
            }
        );

        // Test kilobytes (decimal)
        assert_eq!(
            Quantity::parse("4K").unwrap(),
            Quantity {
                base: 4,
                magnitude: 1,
                power: 3
            }
        );

        // Test megabytes (decimal)
        assert_eq!(
            Quantity::parse("512M").unwrap(),
            Quantity {
                base: 512,
                magnitude: 1,
                power: 6
            }
        );

        // Test gigabytes (decimal)
        assert_eq!(
            Quantity::parse("4G").unwrap(),
            Quantity {
                base: 4,
                magnitude: 1,
                power: 9
            }
        );

        // Test gibibytes (binary)
        assert_eq!(
            Quantity::parse("2Gi").unwrap(),
            Quantity {
                base: 2,
                magnitude: 1,
                power: 30
            }
        );
    }

    #[test]
    fn quantity_display() {
        assert_eq!(Quantity::parse("100").unwrap().to_string(), "100");
        assert_eq!(Quantity::parse("4K").unwrap().to_string(), "4K");
        assert_eq!(Quantity::parse("512M").unwrap().to_string(), "512M");
        assert_eq!(Quantity::parse("4G").unwrap().to_string(), "4G");
        assert_eq!(Quantity::parse("2Gi").unwrap().to_string(), "2Gi");
        assert_eq!(Quantity::parse("1T").unwrap().to_string(), "1T");
    }

    #[test]
    fn quantity_to_bytes() {
        assert_eq!(Quantity::parse("100").unwrap().to_bytes().unwrap(), 100);
        assert_eq!(Quantity::parse("4K").unwrap().to_bytes().unwrap(), 4_000);
        assert_eq!(
            Quantity::parse("512M").unwrap().to_bytes().unwrap(),
            512_000_000
        );
        assert_eq!(
            Quantity::parse("4G").unwrap().to_bytes().unwrap(),
            4_000_000_000
        );
        assert_eq!(
            Quantity::parse("2Gi").unwrap().to_bytes().unwrap(),
            2 * (1i64 << 30)
        );
    }

    #[test]
    fn count_limit_toml() {
        #[derive(Deserialize)]
        struct T {
            n: CountLimit,
        }

        let t: T = toml::from_str("n = 50").unwrap();
        assert_eq!(t.n, CountLimit::Value(50));
        let t: T = toml::from_str(r#"n = "max""#).unwrap();
        assert_eq!(t.n, CountLimit::Max);
        assert!(toml::from_str::<T>("n = -1").is_err());
    }

    #[test]
    fn quantity_serde() {
        // Serialize to JSON
        let q = Quantity::parse("4G").unwrap();
        let json = serde_json::to_string(&q).unwrap();
        assert_eq!(json, r#""4G""#);

        // Deserialize from JSON
        let deserialized: Quantity = serde_json::from_str(r#""8G""#).unwrap();
        assert_eq!(
            deserialized,
            Quantity {
                base: 8,
                magnitude: 1,
                power: 9
            }
        );

        // Round-trip
        let original = Quantity::parse("2Gi").unwrap();
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: Quantity = serde_json::from_str(&serialized).unwrap();
        assert_eq!(original, deserialized);
    }
}
