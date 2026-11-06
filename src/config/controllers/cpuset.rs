use std::{fmt::Display, ops::Range, str::FromStr};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct CpuSet {
    /// CPU configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpus: Option<Cpus>,
    /// Memory node configuration (cpuset.mems)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mems: Option<MemRanges>,
    /// Partition type configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition: Option<PartitionType>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Cpus {
    /// The list of requested CPUs (cpuset.cpus)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpus: Option<CpuRanges>,
    /// The list of exclusive CPUs (cpuset.cpus.exclusive)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclusive: Option<CpuRanges>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PartitionType {
    /// Regular member cgroup (default)
    #[serde(rename = "member")]
    Member,
    /// Root partition cgroup
    #[serde(rename = "root")]
    Root,
    /// Isolated partition cgroup
    #[serde(rename = "isolated")]
    Isolated,
}

impl Display for PartitionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PartitionType::Member => write!(f, "member"),
            PartitionType::Root => write!(f, "root"),
            PartitionType::Isolated => write!(f, "isolated"),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CpuRanges(Vec<Range<u8>>);

impl Display for CpuRanges {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();
        for range in &self.0 {
            if range.start + 1 == range.end {
                parts.push(format!("{}", range.start));
            } else {
                parts.push(format!("{}-{}", range.start, range.end - 1));
            }
        }
        write!(f, "{}", parts.join(","))
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MemRanges(Vec<Range<u8>>);

impl Display for MemRanges {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();
        for range in &self.0 {
            if range.start + 1 == range.end {
                parts.push(format!("{}", range.start));
            } else {
                parts.push(format!("{}-{}", range.start, range.end - 1));
            }
        }
        write!(f, "{}", parts.join(","))
    }
}

impl Serialize for CpuRanges {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut parts = Vec::new();
        for range in &self.0 {
            if range.start + 1 == range.end {
                parts.push(format!("{}", range.start));
            } else {
                parts.push(format!("{}-{}", range.start, range.end - 1));
            }
        }
        let s = parts.join(",");
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for CpuRanges {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        parse_ranges(&s).map(Self).map_err(serde::de::Error::custom)
    }
}

impl Serialize for MemRanges {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut parts = Vec::new();
        for range in &self.0 {
            if range.start + 1 == range.end {
                parts.push(format!("{}", range.start));
            } else {
                parts.push(format!("{}-{}", range.start, range.end - 1));
            }
        }
        let s = parts.join(",");
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for MemRanges {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        parse_ranges(&s).map(Self).map_err(serde::de::Error::custom)
    }
}

/// Parse a cpuset list like "0-3,6,8-10" into half-open ranges.
fn parse_ranges(s: &str) -> Result<Vec<Range<u8>>, Box<dyn std::error::Error>> {
    fn exclusive_end(inclusive: u8) -> Result<u8, Box<dyn std::error::Error>> {
        inclusive
            .checked_add(1)
            .ok_or_else(|| format!("id {inclusive} is out of range").into())
    }

    let mut ranges = Vec::new();
    for part in s.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (start, end) = match part.split_once('-') {
            Some((start, end)) => (start.trim().parse::<u8>()?, end.trim().parse::<u8>()?),
            None => {
                let id = part.parse::<u8>()?;
                (id, id)
            }
        };
        if start > end {
            return Err(format!("invalid range '{part}'").into());
        }
        ranges.push(start..exclusive_end(end)?);
    }
    Ok(ranges)
}

impl CpuRanges {
    /// Create CpuRanges from individual CPU IDs
    pub fn from_cpus(cpus: &[u8]) -> Self {
        let ranges = cpus.iter().map(|&cpu| cpu..cpu.saturating_add(1)).collect();
        Self(ranges)
    }
}

impl From<Range<u8>> for CpuRanges {
    fn from(range: Range<u8>) -> Self {
        Self(vec![range])
    }
}

impl From<Vec<Range<u8>>> for CpuRanges {
    fn from(ranges: Vec<Range<u8>>) -> Self {
        Self(ranges)
    }
}

impl FromStr for CpuRanges {
    type Err = Box<dyn std::error::Error>;

    /// Create CpuRanges from a comma-separated string (e.g., "0-3,6,8-10")
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(parse_ranges(s)?))
    }
}

impl MemRanges {
    /// Create MemRanges from individual memory node IDs
    pub fn from_nodes(nodes: &[u8]) -> Self {
        let ranges = nodes.iter().map(|&node| node..node.saturating_add(1)).collect();
        Self(ranges)
    }
}

impl From<Range<u8>> for MemRanges {
    fn from(range: Range<u8>) -> Self {
        Self(vec![range])
    }
}

impl From<Vec<Range<u8>>> for MemRanges {
    fn from(ranges: Vec<Range<u8>>) -> Self {
        Self(ranges)
    }
}

impl FromStr for MemRanges {
    type Err = Box<dyn std::error::Error>;

    /// Create MemRanges from a comma-separated string (e.g., "0-1,3")
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(parse_ranges(s)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_ranges_serialization() {
        let ranges = CpuRanges::from_str("0-3,6,8-10").unwrap();
        let serialized = serde_json::to_string(&ranges).unwrap();
        assert_eq!(serialized, "\"0-3,6,8-10\"");

        let deserialized: CpuRanges = serde_json::from_str(&serialized).unwrap();
        assert_eq!(ranges, deserialized);
    }

    #[test]
    fn test_mem_ranges_serialization() {
        let ranges = MemRanges::from_str("0-1,3").unwrap();
        let serialized = serde_json::to_string(&ranges).unwrap();
        assert_eq!(serialized, "\"0-1,3\"");

        let deserialized: MemRanges = serde_json::from_str(&serialized).unwrap();
        assert_eq!(ranges, deserialized);
    }

    #[test]
    fn test_partition_type_serialization() {
        let partition = PartitionType::Isolated;
        let serialized = serde_json::to_string(&partition).unwrap();
        assert_eq!(serialized, "\"isolated\"");

        let deserialized: PartitionType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(partition, deserialized);
    }

    #[test]
    fn test_single_cpu_range() {
        let ranges = CpuRanges::from_str("5").unwrap();
        let serialized = serde_json::to_string(&ranges).unwrap();
        assert_eq!(serialized, "\"5\"");
    }

    #[test]
    fn test_from_cpus() {
        let ranges = CpuRanges::from_cpus(&[0, 2, 4]);
        let serialized = serde_json::to_string(&ranges).unwrap();
        assert_eq!(serialized, "\"0,2,4\"");
    }

    #[test]
    fn test_from_range() {
        let ranges = CpuRanges::from(1..4);
        let serialized = serde_json::to_string(&ranges).unwrap();
        assert_eq!(serialized, "\"1-3\"");
    }
}
