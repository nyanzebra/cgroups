use std::{
    borrow::Cow,
    fs::{OpenOptions, create_dir_all, remove_dir},
    io::{ErrorKind, Write as _},
    path::{Path, PathBuf},
    time::Duration,
};

use libc::pid_t;
use log::error;

use crate::config::{
    Config,
    controllers::{
        cpu::{CpuLimits, CpuMax, CpuWeight, Uclamp, UclampMax, UclampMin},
        cpuset::{CpuSet, Cpus},
        hugetlb::{HugetlbLimits, HugetlbPageLimit},
        io::IoLimits,
        mem::MemoryLimits,
        pid::PidsLimits,
    },
};

const CGROUP_BASE: &str = "/sys/fs/cgroup/ash";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cgroup<'a> {
    name: Cow<'a, str>,
}

// https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html
impl Cgroup<'_> {
    pub fn path(&self) -> PathBuf {
        cgroup_path(self.name.as_ref())
    }

    pub fn build(name: &str, config: &Config) -> crate::Result<Self> {
        let Config {
            cpu,
            cpuset,
            memory,
            io,
            pids,
            hugetlb,
            rdma,
        } = config;

        validate_name(name)?;

        // Constructed up front, not at the end: if any write below fails and
        // this function returns early via `?`, this local value still goes
        // out of scope and its `Drop` impl cleans up whatever was already
        // written, instead of leaking a half-configured cgroup.
        let cgroup = Cgroup {
            name: Cow::Owned(name.to_string()),
        };

        create_dir_all(cgroup.path())?;

        if let Some(CpuLimits {
            weight,
            max,
            burst,
            idle,
            uclamp,
        }) = cpu
        {
            if let Some(weight) = weight {
                match weight {
                    CpuWeight::Weight(w) => {
                        create_full_cgroup_path_and_write(name, "cpu.weight", w)?;
                    }
                    CpuWeight::Nice(n) => {
                        create_full_cgroup_path_and_write(name, "cpu.weight.nice", n)?;
                    }
                }
            }

            if let Some(max) = max {
                match max {
                    CpuMax::Limited { quota, period } => {
                        create_full_cgroup_path_and_write(
                            name,
                            "cpu.max",
                            format!("{quota} {period}"),
                        )?;
                    }
                    CpuMax::Max => {
                        create_full_cgroup_path_and_write(name, "cpu.max", "max 100000")?;
                    }
                }
            }

            if let Some(burst) = burst {
                create_full_cgroup_path_and_write(name, "cpu.max.burst", burst)?;
            }

            if let Some(idle) = idle {
                create_full_cgroup_path_and_write(name, "cpu.idle", if *idle { "1" } else { "0" })?;
            }

            if let Some(Uclamp { min, max }) = uclamp {
                match min {
                    UclampMin::Value(val) => {
                        create_full_cgroup_path_and_write(name, "cpu.uclamp.min", val)?
                    }
                    UclampMin::Default => {
                        create_full_cgroup_path_and_write(name, "cpu.uclamp.min", "0")?
                    }
                }

                match max {
                    UclampMax::Value(val) => {
                        create_full_cgroup_path_and_write(name, "cpu.uclamp.max", val)?
                    }
                    UclampMax::Default => {
                        create_full_cgroup_path_and_write(name, "cpu.uclamp.max", "max")?
                    }
                }
            }
        }

        if let Some(MemoryLimits {
            max,
            high,
            low,
            min,
            swap_max,
            swap_high,
            zswap_max,
            oom_group,
        }) = memory
        {
            if let Some(max) = max {
                create_full_cgroup_path_and_write(name, "memory.max", max.to_cgroup_value()?)?;
            }

            if let Some(high) = high {
                create_full_cgroup_path_and_write(name, "memory.high", high.to_cgroup_value()?)?;
            }

            if let Some(low) = low {
                create_full_cgroup_path_and_write(name, "memory.low", low.to_cgroup_value()?)?;
            }

            if let Some(min) = min {
                create_full_cgroup_path_and_write(name, "memory.min", min.to_cgroup_value()?)?;
            }

            if let Some(swap_max) = swap_max {
                create_full_cgroup_path_and_write(
                    name,
                    "memory.swap.max",
                    swap_max.to_cgroup_value()?,
                )?;
            }

            if let Some(swap_high) = swap_high {
                create_full_cgroup_path_and_write(
                    name,
                    "memory.swap.high",
                    swap_high.to_cgroup_value()?,
                )?;
            }

            if let Some(zswap_max) = zswap_max {
                create_full_cgroup_path_and_write(
                    name,
                    "memory.zswap.max",
                    zswap_max.to_cgroup_value()?,
                )?;
            }

            if let Some(oom_group) = oom_group {
                create_full_cgroup_path_and_write(
                    name,
                    "memory.oom.group",
                    if *oom_group { "1" } else { "0" },
                )?;
            }
        }

        if let Some(IoLimits { max, weight }) = io {
            if let Some(max) = max {
                for device in &max.devices {
                    create_full_cgroup_path_and_write(name, "io.max", device)?;
                }
            }

            if let Some(weight) = weight {
                create_full_cgroup_path_and_write(name, "io.weight", weight)?;
            }
        }

        if let Some(PidsLimits { max: Some(max) }) = pids {
            create_full_cgroup_path_and_write(name, "pids.max", max)?;
        }

        if let Some(CpuSet {
            cpus,
            mems,
            partition,
        }) = cpuset
        {
            if let Some(Cpus { cpus, exclusive }) = cpus {
                if let Some(cpus) = cpus {
                    create_full_cgroup_path_and_write(name, "cpuset.cpus", cpus)?;
                }

                if let Some(exclusive) = exclusive {
                    create_full_cgroup_path_and_write(name, "cpuset.cpus.exclusive", exclusive)?;
                }
            }

            if let Some(mems) = mems {
                create_full_cgroup_path_and_write(name, "cpuset.mems", mems)?;
            }

            if let Some(partition) = partition {
                create_full_cgroup_path_and_write(name, "cpuset.partition", partition)?;
            }
        }

        if let Some(rdma) = rdma {
            for (device, limit) in &rdma.limits {
                create_full_cgroup_path_and_write(name, "rdma.max", format!("{device} {limit}"))?;
            }
        }

        if let Some(HugetlbLimits { limits }) = hugetlb {
            for HugetlbPageLimit { page_size, max } in limits {
                if let Some(max) = max {
                    create_full_cgroup_path_and_write(
                        name,
                        format!("hugetlb.{}.max", page_size),
                        max.to_cgroup_value()?,
                    )?;
                }
            }
        }

        Ok(cgroup)
    }

    pub fn add(&self, pid: pid_t) -> crate::Result<()> {
        create_cgroup(pid, self.name.as_ref())
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Drop for Cgroup<'_> {
    fn drop(&mut self) {
        let result = cleanup_cgroup(self.name.as_ref());
        if let Err(err) = result {
            error!("failed to cleanup cgroup for '{}': '{err}'", self.name);
        }
    }
}

/// Reject names that would escape `CGROUP_BASE` (absolute paths, `..`) or
/// nest/alias other cgroups.
fn validate_name(name: &str) -> crate::Result<()> {
    if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\0']) {
        return Err(crate::Error::InvalidName(name.to_string()));
    }
    Ok(())
}

fn create_cgroup<P>(pid: pid_t, name: P) -> crate::Result<()>
where
    P: AsRef<Path>,
{
    create_full_file_path_and_write(name.as_ref().join("cgroup.procs"), pid.to_string())?;
    Ok(())
}

fn cleanup_cgroup<P>(name: P) -> crate::Result<()>
where
    P: AsRef<Path>,
{
    let dir = cgroup_path(name);

    match OpenOptions::new().write(true).open(dir.join("cgroup.kill")) {
        Ok(mut file) => file.write_all(b"1\n")?,
        Err(err) if err.kind() == ErrorKind::NotFound => {}
        Err(err) => return Err(err.into()),
    }

    // cgroup.kill is asynchronous; rmdir fails with EBUSY until the
    // killed processes have actually exited.
    let mut attempts = 0;
    loop {
        match remove_dir(&dir) {
            Ok(()) => return Ok(()),
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(()),
            Err(err) if err.raw_os_error() == Some(libc::EBUSY) && attempts < 50 => {
                attempts += 1;
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(err) => return Err(err.into()),
        }
    }
}

fn create_full_cgroup_path_and_write<P, Q, S>(sub: P, name: Q, content: S) -> crate::Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
    S: ToString,
{
    create_full_file_path_and_write(sub.as_ref().join(name), content.to_string())
}

fn create_full_file_path_and_write<P, S>(path: P, content: S) -> crate::Result<()>
where
    P: AsRef<Path>,
    S: AsRef<str>,
{
    let file_path = cgroup_path(path);

    // Only the cgroup directory itself needs to be created — the kernel
    // auto-populates every control file (memory.max, cgroup.procs, etc.) as
    // soon as the directory exists. `create_dir_all` must never be called on
    // `file_path` itself, or it turns the control file into a directory.
    if let Some(parent) = file_path.parent() {
        create_dir_all(parent)?;
    }

    // Control files must get exactly one write() per value; `writeln!` can
    // split the value and the newline into separate syscalls, which the
    // kernel may reject. A temp-file unit test can't catch this.
    let mut file = OpenOptions::new().write(true).open(&file_path)?;
    file.write_all(format!("{}\n", content.as_ref()).as_bytes())?;
    Ok(())
}

fn cgroup_path<P>(name: P) -> PathBuf
where
    P: AsRef<Path>,
{
    PathBuf::from(CGROUP_BASE).join(name.as_ref())
}
