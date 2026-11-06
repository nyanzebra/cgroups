//! Same shape of cgroup as `create_cgroup.rs`, but described in
//! `examples/cgroup.toml` and loaded via `Deserialize` instead of being
//! built directly in Rust.
//!
//!     cargo build --example create_cgroup_from_toml
//!     sudo ./target/debug/examples/create_cgroup_from_toml
//! examples/cgroup.toml

use cgroups::{Config, interface::v2::Cgroup};

fn main() -> cgroups::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: create_cgroup_from_toml <path/to/cgroup.toml>");

    let contents = std::fs::read_to_string(&path)?;
    let config: Config =
        toml::from_str(&contents).unwrap_or_else(|err| panic!("failed to parse '{path}': {err}"));

    let cgroup = Cgroup::build("example-toml", &config)?;
    println!("created cgroup at {}", cgroup.path().display());

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
