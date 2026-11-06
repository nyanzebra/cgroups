use serde::{Deserialize, Serialize};

use crate::QuantityLimit;

/// Hugetlb controller configuration for cgroup v2
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct HugetlbLimits {
    /// Hugetlb limits per page size
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub limits: Vec<HugetlbPageLimit>,
}

/// Hugetlb limit for a specific page size
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HugetlbPageLimit {
    /// Page size (e.g., "2MB", "1GB")
    pub page_size: String,
    /// Maximum hugetlb usage (hugetlb.<size>.max)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<QuantityLimit>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hugetlb_limit_max_serialization() {
        let limit = QuantityLimit::Max;
        let serialized = serde_json::to_string(&limit).unwrap();
        assert_eq!(serialized, "\"max\"");

        let deserialized: QuantityLimit = serde_json::from_str(&serialized).unwrap();
        assert_eq!(limit, deserialized);
    }

    #[test]
    fn test_empty_hugetlb_limits() {
        let limits = HugetlbLimits::default();
        let serialized = serde_json::to_string(&limits).unwrap();
        assert_eq!(serialized, "{}");
    }
}
