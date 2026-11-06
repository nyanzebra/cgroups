use serde::{Deserialize, Serialize};

use crate::CountLimit;

/// PID controller configuration for cgroup v2
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct PidsLimits {
    /// Maximum number of processes/threads allowed ("max" for unlimited)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<CountLimit>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pids_limits_serialization() {
        let limits = PidsLimits {
            max: Some(CountLimit::Value(1000)),
        };

        let serialized = serde_json::to_string(&limits).unwrap();
        let deserialized: PidsLimits = serde_json::from_str(&serialized).unwrap();

        assert_eq!(limits, deserialized);
    }

    #[test]
    fn test_pid_limit_max_serialization() {
        let limit = CountLimit::Max;
        let serialized = serde_json::to_string(&limit).unwrap();
        assert_eq!(serialized, "\"max\"");

        let deserialized: CountLimit = serde_json::from_str(&serialized).unwrap();
        assert_eq!(limit, deserialized);
    }

    #[test]
    fn test_pid_limit_value_serialization() {
        let limit = CountLimit::Value(1000);
        let serialized = serde_json::to_string(&limit).unwrap();
        assert_eq!(serialized, "1000");

        let deserialized: CountLimit = serde_json::from_str(&serialized).unwrap();
        assert_eq!(limit, deserialized);
    }

    #[test]
    fn test_empty_pids_limits() {
        let limits = PidsLimits::default();
        let serialized = serde_json::to_string(&limits).unwrap();
        assert_eq!(serialized, "{}");
    }
}
