# TTstack — Lightweight Private Cloud

TTstack (Temporary Test Stack) is a lightweight private cloud platform designed
for mid-size teams and individual developers. It provides simple, centralized
management of virtual machines and containers across multiple physical hosts.

> **The Efficiency Paradox of SMEs**
>
> Start-up SMEs typically have weak technical infrastructure, with production
> methods still stuck in manual processes. While they may excel at *human*
> efficiency (management, agility), they fall behind on *tool* efficiency.
> OpenStack and Kubernetes are powerful but demand expensive, specialized teams
> to operate. TTstack solves this: a sysadmin can deploy it in **30 minutes**,
> and end users learn it in **10 minutes**.

## Key Features

- **Multi-engine**: QEMU/KVM, Firecracker, Bhyve (FreeBSD), Docker/Podman
- **Multi-host fleet**: manage up to 50 physical hosts and 1000 VM instances
- **Native filesystem support**: ZFS snapshots, Btrfs subvolumes, or raw file copies
- **Environments**: group related VMs with lifecycle control (auto-expiry, stop/start)
- **Web dashboard**: built-in browser-based management UI (no extra services)
- **Simple deployment**: three static binaries, one SQLite database, minimal dependencies
- **HTTP REST API**: straightforward JSON API for automation and CI/CD integration

## Architecture

```
                         ┌──────────────┐
                         │   Web  UI    │
                         └──────┬───────┘
                                │
┌──────────┐     HTTP    ┌──────┴───────┐     HTTP    ┌───────────┐
│  tt CLI  ├────────────►│   tt-ctl     ├────────────►│ tt-agent  │ × N
└──────────┘             │ (controller) │             │ (per-host)│
                         └──────┬───────┘             └─────┬─────┘
                                │                           │
                           SQLite DB                Local VM engines
                                                 (qemu / fc / docker)
```

| Binary | Role |
|--------|------|
| **tt** | CLI client |
| **tt-ctl** | Central controller: scheduling, state, web UI |
| **tt-agent** | Host agent: VM lifecycle, images, networking |

## Quick Start

### 1. Build and install

```bash
cargo build --release

# Create dedicated user and install
useradd -r -m -d /home/ttstack -s /bin/bash ttstack
mkdir -p /opt/ttstack/bin
cp target/release/{tt,tt-ctl,tt-agent} /opt/ttstack/bin/
chown -R ttstack:ttstack /home/ttstack
```

Directory layout:
```
/opt/ttstack/bin/          # binaries (tt, tt-ctl, tt-agent)
/home/ttstack/             # runtime data (ttstack user home)
  ├── images/              # base VM/container images
  ├── runtime/             # VM image clones (transient)
  ├── data/                # agent SQLite database
  ├── ctl/                 # controller SQLite database
  └── run/                 # PID files, sockets
```

### 2. Start an agent on each host

```bash
# Run as the ttstack user (all paths default to /home/ttstack/*)
su - ttstack -c '/opt/ttstack/bin/tt-agent \
  --listen 0.0.0.0:9100 \
  --storage raw \
  --cpu-total 16 \
  --mem-total 32768 \
  --disk-total 500000'
```

### 3. Start the controller

```bash
su - ttstack -c '/opt/ttstack/bin/tt-ctl --listen 0.0.0.0:9200'
```

### 4. Register hosts and create environments

```bash
tt config 10.0.0.1:9200             # point CLI to controller
tt host add 10.0.0.2:9100           # register host
tt host list                        # verify

tt env create my-test \
  --image ubuntu-22.04 \
  --image centos-9 \
  --engine qemu \
  --cpu 2 --mem 2048 \
  --dup 2 \
  --port 22 --port 80

tt env show my-test                 # see VM details
tt env stop my-test                 # pause all VMs
tt env start my-test                # resume
tt env delete my-test               # destroy everything
```

### 5. Web dashboard

Open `http://<controller-addr>:9200` in a browser.

## CLI Reference

```
tt config <controller-addr>         Set controller address
tt status                           Fleet-wide status

tt host add <agent-addr>            Register a host
tt host list                        List all hosts
tt host show <id>                   Show host details
tt host remove <id>                 Unregister a host

tt env create <name> [opts]         Create environment with VMs
tt env list                         List environments
tt env show <name>                  Show environment + VM details
tt env delete <name>                Destroy environment
tt env stop <name>                  Stop all VMs
tt env start <name>                 Start all VMs

tt image list                       List images across all hosts
```

### `env create` options

| Option | Description | Default |
|--------|-------------|---------|
| `-i, --image <name>` | Base image (repeatable) | *required* |
| `--engine <type>` | qemu, firecracker, docker, bhyve, jail | qemu |
| `--cpu <N>` | vCPUs per VM | 2 |
| `--mem <MiB>` | Memory per VM in MiB | 1024 (1 GiB) |
| `--disk <MiB>` | Disk per VM in MiB | 40960 (40 GiB) |
| `--dup <N>` | Replicas per image | 1 |
| `-p, --port <PORT>` | Guest port to expose (repeatable) | — |
| `--lifetime <SEC>` | Auto-expiry in seconds | 21600 (6h) |
| `--deny-outgoing` | Block outbound traffic | false |
| `--owner <USER>` | Owner label | `$USER` |

## Storage Backends

| Backend | How it works | When to use |
|---------|-------------|-------------|
| **zfs** | ZFS snapshot + clone | Production (instant, space-efficient) |
| **btrfs** | Btrfs subvolume snapshot | Production (instant, space-efficient) |
| **raw** | `cp --reflink=auto` | Development, any filesystem |

```bash
# ZFS example
tt-agent --storage zfs --image-dir tank/ttstack/images

# Btrfs example
tt-agent --storage btrfs --image-dir /mnt/btrfs/images

# Raw (default, works everywhere)
tt-agent --storage raw --image-dir /home/ttstack/images
```

## Agent Configuration

```
tt-agent [OPTIONS]

  --listen <ADDR>         Listen address          [0.0.0.0:9100]
  --image-dir <PATH>      Base image directory     [/home/ttstack/images]
  --runtime-dir <PATH>    Runtime clone directory  [/home/ttstack/runtime]
  --data-dir <PATH>       Database directory       [/home/ttstack/data]
  --storage <TYPE>        zfs | btrfs | raw        [raw]
  --cpu-total <N>         CPU cores (0=auto)       [0]
  --mem-total <N>         Memory MB (0=auto)       [0]
  --disk-total <N>        Disk MB                  [200000]
  --host-id <ID>          Host ID (auto if omit)
```

## Controller Configuration

```
tt-ctl [OPTIONS]

  --listen <ADDR>       Listen address          [0.0.0.0:9200]
  --data-dir <PATH>     Database directory       [/home/ttstack/ctl]
```

## REST API

### Controller endpoints (used by CLI and web UI)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/` | Web dashboard |
| POST | `/api/hosts` | Register a host `{"addr":"..."}` |
| GET | `/api/hosts` | List hosts |
| GET | `/api/hosts/{id}` | Host details |
| DELETE | `/api/hosts/{id}` | Remove host |
| POST | `/api/envs` | Create environment |
| GET | `/api/envs` | List environments |
| GET | `/api/envs/{id}` | Environment + VM details |
| DELETE | `/api/envs/{id}` | Destroy environment |
| POST | `/api/envs/{id}/stop` | Stop environment |
| POST | `/api/envs/{id}/start` | Start environment |
| GET | `/api/images` | List images across fleet |
| GET | `/api/status` | Fleet-wide resource status |

### Agent endpoints (used by controller)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/info` | Host info and resources |
| GET | `/api/images` | Available images |
| POST | `/api/vms` | Create a VM |
| GET | `/api/vms` | List VMs |
| GET | `/api/vms/{id}` | VM details |
| DELETE | `/api/vms/{id}` | Destroy VM |
| POST | `/api/vms/{id}/stop` | Stop VM |
| POST | `/api/vms/{id}/start` | Start VM |

## System Requirements

**Controller host**: Linux (any distro), Rust 1.86+

**Agent hosts**:
- Linux x86_64 (for QEMU, Firecracker, Docker)
- FreeBSD (for Bhyve)
- One or more engines installed
- nftables (for NAT and port forwarding)
- Kernel modules: `tun`, `vhost_net`, `kvm_intel` / `kvm_amd`

**Preparing a QEMU image**:
1. Install your OS in a qcow2 image
2. Ensure DHCP or static IP configured for `10.10.0.0/16`
3. Disable firewall and SELinux inside the guest
4. Place the image file in the agent's `--image-dir`

## Project Structure

```
TTstack/
├── Cargo.toml              Workspace root
├── README.md
└── crates/
    ├── core/               Shared library
    │   └── src/
    │       ├── lib.rs
    │       ├── model.rs    Data models (Host, Vm, Env, Engine, Storage)
    │       ├── api.rs      Request/response types
    │       ├── net.rs      Network utilities (bridge, TAP, nftables)
    │       ├── engine/     VM engine trait + implementations
    │       │   ├── mod.rs      VmEngine trait, factory
    │       │   ├── qemu.rs     QEMU/KVM
    │       │   ├── firecracker.rs
    │       │   ├── bhyve.rs    FreeBSD only
    │       │   └── docker.rs   Docker/Podman
    │       └── storage/    Image storage backends
    │           ├── mod.rs      ImageStore trait, factory
    │           ├── zfs.rs
    │           ├── btrfs.rs
    │           └── raw.rs
    ├── agent/              Host agent (tt-agent)
    │   └── src/
    │       ├── main.rs     Entry point
    │       ├── config.rs   CLI config (clap)
    │       ├── handler.rs  HTTP handlers
    │       └── runtime.rs  VM lifecycle + SQLite state
    ├── ctl/                Controller (tt-ctl)
    │   └── src/
    │       ├── main.rs     Entry point + env expiry task
    │       ├── config.rs   CLI config (clap)
    │       ├── handler.rs  HTTP handlers
    │       ├── scheduler.rs VM placement (best-fit)
    │       ├── db.rs       SQLite state management
    │       └── web.rs      Embedded web dashboard
    └── cli/                CLI client (tt)
        └── src/
            ├── main.rs     Command definitions (clap)
            └── client.rs   HTTP client + config file
```

## License

MIT OR Apache-2.0
