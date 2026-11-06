use serde::{Deserialize, Serialize};

pub mod controllers;
use controllers::{
    cpu::CpuLimits, cpuset::CpuSet, hugetlb::HugetlbLimits, io::IoLimits, mem::MemoryLimits,
    pid::PidsLimits, rdma::RdmaLimits,
};

/// Cgroup v2 resource limits configuration
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Config {
    /// CPU controller limits
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<CpuLimits>,

    /// CPUSet controller limits
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpuset: Option<CpuSet>,

    /// Memory controller limits
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<MemoryLimits>,

    /// I/O controller limits
    #[serde(skip_serializing_if = "Option::is_none")]
    pub io: Option<IoLimits>,

    /// Process/thread limits
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pids: Option<PidsLimits>,

    /// Hugetlb controller limits
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hugetlb: Option<HugetlbLimits>,

    /// RDMA controller limits
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rdma: Option<RdmaLimits>,
}
