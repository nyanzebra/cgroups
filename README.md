# cgroups

A library for describing Linux [cgroup v2](https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html)
resource limits as a serializable `Config`, and applying them by creating a
real cgroup on disk under `/sys/fs/cgroup/ash/<name>`.

```rust
use cgroups::config::controllers::mem::MemoryLimits;
use cgroups::interface::v2::Cgroup;
use cgroups::{Config, Quantity, QuantityLimit};

let config = Config {
    memory: Some(MemoryLimits {
        max: Some(QuantityLimit::Value(Quantity::parse("512M")?)),
        ..Default::default()
    }),
    ..Default::default()
};

let cgroup = Cgroup::build("my-service", &config)?;
cgroup.add(child_pid)?;
// cgroup is torn down (and everything still running in it killed)
// when `cgroup` is dropped.
```

## Prerequisites

- **Root.** Every write goes to `/sys/fs/cgroup/...`.
- **cgroup v2 unified hierarchy mounted** (the default on any recent
  distribution; check with `stat -f -c %T /sys/fs/cgroup`, which should
  print `cgroup2fs`).
- **The parent cgroup must already have the relevant controllers enabled.**
  `Cgroup::build` enables controllers (`+cpu`, `+memory`, etc.) in
  `<name>/cgroup.subtree_control` — but that only turns them on for
  `<name>`'s *children*. For `<name>`'s own `memory.max`/`cpu.max`/etc. files
  to exist at all, `/sys/fs/cgroup/ash` (and everything above it, up to the
  true root) needs those same controllers enabled in *its own*
  `cgroup.subtree_control` first. This crate doesn't create or bootstrap
  that base directory — set it up once yourself:
  ```bash
  mkdir -p /sys/fs/cgroup/ash
  echo "+cpu +memory +pids +io +cpuset +hugetlb +rdma" > /sys/fs/cgroup/ash/cgroup.subtree_control
  ```
  (and make sure the real root's `cgroup.subtree_control` already has those
  controllers too, which it does by default on most systems).

## Units

Config values use human-readable quantities (`"512M"`, `"4Gi"`, parsed by
`Quantity`), but cgroup v2 control files take raw byte counts — the kernel
doesn't parse unit suffixes at all. `QuantityLimit` (used by every
memory/hugetlb limit) resolves this via `to_cgroup_value()`, which is what
actually gets written; the human-readable form is only for config files and
`Display`.

`Quantity::parse` accepts:

| Suffix          | Meaning                    |
|-----------------|----------------------------|
| (none), `B`     | bytes                      |
| `K`, `KB`       | kilobytes (10³)            |
| `Ki`, `KiB`     | kibibytes (2¹⁰)            |
| `M`, `MB`       | megabytes (10⁶)            |
| `Mi`, `MiB`     | mebibytes (2²⁰)            |
| `G`, `GB`       | gigabytes (10⁹)            |
| `Gi`, `GiB`     | gibibytes (2³⁰)            |
| `T`, `TB`       | terabytes (10¹²)           |
| `Ti`, `TiB`     | tebibytes (2⁴⁰)            |

## Cleanup

`Cgroup` removes its own cgroup when dropped: it writes `1` to
`cgroup.kill` (killing anything still running inside it) and then removes
the directory. This also means a `Cgroup` that fails partway through
`build()` still cleans up whatever had already been written, rather than
leaking a half-configured cgroup.

## Examples

[`examples/create_cgroup.rs`](examples/create_cgroup.rs) builds a `Config`
directly in Rust:

```bash
cargo build --example create_cgroup
sudo ./target/debug/examples/create_cgroup
```

[`examples/create_cgroup_from_toml.rs`](examples/create_cgroup_from_toml.rs)
loads the same shape of config from
[`examples/cgroup.toml`](examples/cgroup.toml) via `Deserialize`:

```bash
cargo build --example create_cgroup_from_toml
sudo ./target/debug/examples/create_cgroup_from_toml examples/cgroup.toml
```

Both spawn a short-lived `sleep` child, add it to the cgroup, wait a couple
seconds, then drop the guard — which kills the child via `cgroup.kill` and
removes the cgroup, so you can watch the whole lifecycle end to end.

## Known limitations

- `CpuRanges` and `MemRanges` (`cpuset.cpus`/`cpuset.mems`) are
  near-identical, independently-implemented parse/format logic. Not a
  correctness issue, just duplicated maintenance — worth unifying at some
  point.
- No automatic bootstrap of the base `/sys/fs/cgroup/ash` cgroup and its
  `cgroup.subtree_control` — see Prerequisites above.
