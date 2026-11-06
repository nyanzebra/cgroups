use serde::{Deserialize, Serialize};

/// CPU weight
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CpuWeight {
    /// Nice cpu weight, range -20 to 19
    Nice(i8),
    /// Weighted cpu weight, range 1 to 10000
    Weight(u16),
}

/// CPU bandwidth limit
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum CpuMax {
    /// Maximum CPU ("max")
    #[default]
    Max,
    /// Limited bandwidth with quota and period
    Limited {
        /// CPU quota in microseconds (e.g., 100000 = 1 core with period 100000)
        quota: u64,
        /// Period in microseconds (typically 100000 = 100ms)
        period: u64,
    },
}

impl PartialEq for UclampMax {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (UclampMax::Default, UclampMax::Default) => true,
            (UclampMax::Value(a), UclampMax::Value(b)) => f32_eq(a, b),
            _ => false,
        }
    }
}

impl Eq for UclampMax {}

/// Uclamp min
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub enum UclampMin {
    /// Minimum CPU utilization (%)
    Value(f32),
    /// Default of 0
    #[default]
    Default,
}

impl Eq for UclampMin {}

impl PartialEq for UclampMin {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (UclampMin::Default, UclampMin::Default) => true,
            (UclampMin::Value(a), UclampMin::Value(b)) => f32_eq(a, b),
            _ => false,
        }
    }
}

fn f32_eq(a: &f32, b: &f32) -> bool {
    (a - b).abs() < f32::EPSILON
}

/// Uclamp max
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub enum UclampMax {
    /// Maximum CPU utilization (%)
    Value(f32),
    /// Default of unlimited (max cpus)
    #[default]
    Default,
}

/// Uclamp
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Uclamp {
    /// Minimum CPU weight
    pub min: UclampMin,
    /// Maximum CPU weight
    pub max: UclampMax,
}

/// CPU controller configuration for cgroup v2
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct CpuLimits {
    /// CPU weight (cpu.weight) - range 1-10000, default 100
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<CpuWeight>,

    /// Maximum CPU bandwidth (cpu.max)
    /// Format: "quota period" e.g. "100000 100000" for 100% of 1 core
    /// "200000 100000" for 200% (2 cores), "max" for unlimited
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<CpuMax>,

    /// CPU burst allowance (cpu.max.burst)
    /// Range: 0-1000000000 (default 0)
    /// Higher values allow more CPU time during bursts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub burst: Option<u64>,

    /// CPU utilization clamping (cpu.uclamp.min, cpu.uclamp.max)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uclamp: Option<Uclamp>,

    /// CPU idle behavior (cpu.idle)
    /// 0: normal behavior, 1: prefer to run tasks on idle CPUs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle: Option<bool>,
}

impl From<u16> for CpuWeight {
    fn from(value: u16) -> Self {
        Self::Weight(value)
    }
}

impl From<i8> for CpuWeight {
    fn from(value: i8) -> Self {
        Self::Nice(value)
    }
}

impl CpuMax {
    /// Create unlimited CPU bandwidth
    pub fn unlimited() -> Self {
        Self::Max
    }

    /// Create limited CPU bandwidth with quota and period
    pub fn limited(quota: u64, period: u64) -> Self {
        Self::Limited { quota, period }
    }

    /// Create CPU limit for a percentage of cores (e.g., 1.5 cores = 150%)
    pub fn from_cores(cores: f64) -> Self {
        let period = 100_000; // Standard 100ms period
        let quota = (cores * period as f64) as u64;
        Self::Limited { quota, period }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_limits_serialization() {
        let limits = CpuLimits {
            weight: Some(CpuWeight::Weight(1000)),
            max: Some(CpuMax::Limited {
                quota: 200000,
                period: 100000,
            }),
            burst: Some(1000000),
            uclamp: Some(Uclamp {
                min: UclampMin::Value(10.0),
                max: UclampMax::Value(80.0),
            }),
            idle: Some(true),
        };

        let serialized = serde_json::to_string(&limits).unwrap();
        let deserialized: CpuLimits = serde_json::from_str(&serialized).unwrap();

        assert_eq!(limits, deserialized);
    }

    #[test]
    fn test_cpu_weight_serialization() {
        let weight = CpuWeight::Weight(1000);
        let serialized = serde_json::to_string(&weight).unwrap();
        let deserialized: CpuWeight = serde_json::from_str(&serialized).unwrap();

        assert_eq!(weight, deserialized);
    }

    #[test]
    fn test_cpu_max_unlimited() {
        let max = CpuMax::Max;
        let serialized = serde_json::to_string(&max).unwrap();
        let deserialized: CpuMax = serde_json::from_str(&serialized).unwrap();

        assert_eq!(max, deserialized);
    }

    #[test]
    fn test_cpu_max_limited() {
        let max = CpuMax::Limited {
            quota: 150000,
            period: 100000,
        };
        let serialized = serde_json::to_string(&max).unwrap();
        let deserialized: CpuMax = serde_json::from_str(&serialized).unwrap();

        assert_eq!(max, deserialized);
    }

    #[test]
    fn test_cpu_max_from_cores() {
        let max = CpuMax::from_cores(2.0);
        assert_eq!(
            max,
            CpuMax::Limited {
                quota: 200000,
                period: 100000
            }
        );
    }

    #[test]
    fn test_uclamp_equality() {
        let uclamp1 = Uclamp {
            min: UclampMin::Value(10.0),
            max: UclampMax::Value(90.0),
        };
        let uclamp2 = Uclamp {
            min: UclampMin::Value(10.0),
            max: UclampMax::Value(90.0),
        };

        assert_eq!(uclamp1, uclamp2);
    }
}
