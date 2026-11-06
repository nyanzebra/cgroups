use serde::{Deserialize, Serialize};

use crate::QuantityLimit;

/// Memory controller configuration for cgroup v2
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct MemoryLimits {
    /// Hard memory limit (memory.max)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<QuantityLimit>,

    /// Memory reclaim threshold (memory.high) - soft limit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub high: Option<QuantityLimit>,

    /// Memory protection threshold (memory.low) - best-effort protection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub low: Option<QuantityLimit>,

    /// Memory hard protection threshold (memory.min) - hard guarantee
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<QuantityLimit>,

    /// Swap usage limit (memory.swap.max)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swap_max: Option<QuantityLimit>,

    /// Swap reclaim threshold (memory.swap.high)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swap_high: Option<QuantityLimit>,

    /// zswap usage limit (memory.zswap.max)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zswap_max: Option<QuantityLimit>,

    /// Kill all processes in cgroup on OOM (memory.oom.group)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oom_group: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Quantity;

    fn create_test_quantity() -> Quantity {
        Quantity {
            base: 1,
            magnitude: 1,
            power: 9,
        } // 1GB
    }

    #[test]
    fn test_memory_limits_serialization() {
        let limits = MemoryLimits {
            max: Some(QuantityLimit::Value(create_test_quantity())),
            high: Some(QuantityLimit::Value(create_test_quantity())),
            low: Some(QuantityLimit::Value(create_test_quantity())),
            min: Some(QuantityLimit::Value(create_test_quantity())),
            swap_max: Some(QuantityLimit::Max),
            swap_high: None,
            zswap_max: None,
            oom_group: Some(true),
        };

        let serialized = serde_json::to_string(&limits).unwrap();
        let deserialized: MemoryLimits = serde_json::from_str(&serialized).unwrap();

        assert_eq!(limits, deserialized);
    }

    #[test]
    fn test_memory_limit_max_serialization() {
        let limit = QuantityLimit::Max;
        let serialized = serde_json::to_string(&limit).unwrap();
        assert_eq!(serialized, "\"max\"");

        let deserialized: QuantityLimit = serde_json::from_str(&serialized).unwrap();
        assert_eq!(limit, deserialized);
    }

    #[test]
    fn test_memory_limit_value_serialization() {
        let limit = QuantityLimit::Value(create_test_quantity());
        let serialized = serde_json::to_string(&limit).unwrap();

        let deserialized: QuantityLimit = serde_json::from_str(&serialized).unwrap();
        assert_eq!(limit, deserialized);
    }

    #[test]
    fn test_memory_limit_to_cgroup_value() {
        assert_eq!(QuantityLimit::Max.to_cgroup_value().unwrap(), "max");
        assert_eq!(
            QuantityLimit::Value(create_test_quantity())
                .to_cgroup_value()
                .unwrap(),
            "1000000000" // 1GB in bytes, not "1G"
        );
    }

    #[test]
    fn test_empty_memory_limits() {
        let limits = MemoryLimits::default();
        let serialized = serde_json::to_string(&limits).unwrap();
        assert_eq!(serialized, "{}");
    }
}
