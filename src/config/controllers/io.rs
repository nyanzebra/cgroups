use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::CountLimit;

/// IO controller configuration for cgroup v2
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct IoLimits {
    /// IO bandwidth and IOPS limits per device
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<IoMax>,
    /// IO weight (1-10000, default 100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<u16>,
}

/// IO bandwidth and IOPS limits
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct IoMax {
    /// Per-device IO limits
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub devices: Vec<IoDeviceLimit>,
}

/// IO limits for a specific device
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IoDeviceLimit {
    /// Device major:minor (e.g., "8:0" for /dev/sda)
    pub device: String,
    /// Read bytes per second limit ("max" for unlimited)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rbps: Option<CountLimit>,
    /// Write bytes per second limit ("max" for unlimited)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wbps: Option<CountLimit>,
    /// Read IOPS limit ("max" for unlimited)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub riops: Option<CountLimit>,
    /// Write IOPS limit ("max" for unlimited)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wiops: Option<CountLimit>,
}

impl Display for IoDeviceLimit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Matches the kernel's expected format for a single write to
        // io.max, e.g. "8:16 rbps=2097152 wbps=1048576" — note no colon
        // after the device, and each device must be written separately
        // (io.max is a nested-keyed file, one line/write per device).
        write!(f, "{}", self.device)?;
        if let Some(rbps) = &self.rbps {
            write!(f, " rbps={}", rbps)?;
        }
        if let Some(wbps) = &self.wbps {
            write!(f, " wbps={}", wbps)?;
        }
        if let Some(riops) = &self.riops {
            write!(f, " riops={}", riops)?;
        }
        if let Some(wiops) = &self.wiops {
            write!(f, " wiops={}", wiops)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_limit_max_serialization() {
        let limit = CountLimit::Max;
        let serialized = serde_json::to_string(&limit).unwrap();
        assert_eq!(serialized, "\"max\"");

        let deserialized: CountLimit = serde_json::from_str(&serialized).unwrap();
        assert_eq!(limit, deserialized);
    }

    #[test]
    fn test_io_limit_value_serialization() {
        let limit = CountLimit::Value(1000000);
        let serialized = serde_json::to_string(&limit).unwrap();
        assert_eq!(serialized, "1000000");

        let deserialized: CountLimit = serde_json::from_str(&serialized).unwrap();
        assert_eq!(limit, deserialized);
    }

    #[test]
    fn test_empty_io_limits() {
        let limits = IoLimits::default();
        let serialized = serde_json::to_string(&limits).unwrap();
        assert_eq!(serialized, "{}");
    }
}
