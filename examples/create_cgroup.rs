//! Build a Config in Rust, create a cgroup from it, add a spawned child
//! process to it, then let the guard's Drop impl clean everything up when
//! it goes out of scope (which also kills anything still running in the
//! cgroup, via cgroup.kill).
//!
//! Requires root and a cgroup v2 filesystem mounted at /sys/fs/cgroup, with
//! the parent cgroup already set up (see the README's Prerequisites
//! section):
//!
//!     cargo build --example create_cgroup
//!     sudo ./target/debug/examples/create_cgroup

use cgroups::{
    Config, CountLimit, Quantity, QuantityLimit,
    config::controllers::{
        cpu::{CpuLimits, CpuMax, CpuWeight},
        mem::MemoryLimits,
        pid::PidsLimits,
    },
    interface::v2::Cgroup,
};

fn main() -> cgroups::Result<()> {
    let config = Config {
        cpu: Some(CpuLimits {
            weight: Some(CpuWeight::Weight(500)),
            max: Some(CpuMax::Limited {
                quota: 150_000,
                period: 100_000,
            }), // 1.5 cores
            ..Default::default()
        }),
        memory: Some(MemoryLimits {
            max: Some(QuantityLimit::Value(Quantity::parse("512M")?)),
            high: Some(QuantityLimit::Value(Quantity::parse("400M")?)),
            ..Default::default()
        }),
        pids: Some(PidsLimits {
            max: Some(CountLimit::Value(100)),
        }),
        ..Default::default()
    };

    let cgroup = Cgroup::build("example", &config)?;
    println!("created cgroup at {}", cgroup.path().display());

    // Spawn a throwaway child rather than adding our own pid — cleanup below
    // sends SIGKILL to every process still in the cgroup, and we don't want
    // that to be this process.
    let mut child = std::process::Command::new("sleep")
        .arg("30")
        .spawn()
        .expect("failed to spawn sleep");

    cgroup.add(child.id() as libc::pid_t)?;
    println!("added child pid {} to the cgroup", child.id());

    std::thread::sleep(std::time::Duration::from_secs(2));

    println!("tearing down the cgroup (this kills the child)...");
    drop(cgroup);

    let status = child.wait().expect("failed to wait on child");
    println!("child exited: {status}");

    Ok(())
}
