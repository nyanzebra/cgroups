use std::{collections::HashMap, fmt::Display};

use serde::{Deserialize, Serialize};

use crate::CountLimit;

/// RDMA controller configuration for cgroup v2
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct RdmaLimits {
    /// RDMA resource limits per device
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub limits: HashMap<String, RdmaDeviceLimit>,
}

/// RDMA resource limits for a specific device
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct RdmaDeviceLimit {
    /// Maximum RDMA handles
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hca_handle: Option<CountLimit>,
    /// Maximum RDMA objects
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hca_object: Option<CountLimit>,
}

impl Display for RdmaDeviceLimit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(hca_handle) = &self.hca_handle {
            write!(f, "hca_handle={} ", hca_handle)?;
        }
        if let Some(hca_object) = &self.hca_object {
            write!(f, "hca_object={} ", hca_object)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rdma_limit_max_serialization() {
        let limit = CountLimit::Max;
        let serialized = serde_json::to_string(&limit).unwrap();
        assert_eq!(serialized, "\"max\"");

        let deserialized: CountLimit = serde_json::from_str(&serialized).unwrap();
        assert_eq!(limit, deserialized);
    }

    #[test]
    fn test_empty_rdma_limits() {
        let limits = RdmaLimits::default();
        let serialized = serde_json::to_string(&limits).unwrap();
        assert_eq!(serialized, "{}");
    }
}
