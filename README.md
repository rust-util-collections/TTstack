# TTstack — Lightweight Private Cloud

[![CI](https://github.com/rust-util-collections/TTstack/actions/workflows/ci.yml/badge.svg)](https://github.com/rust-util-collections/TTstack/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.86%2B-orange.svg)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/platform-linux%20%7C%20freebsd-green.svg)](#platform-support)

TTstack is a lightweight private cloud platform for mid-size teams and
individual developers. Centralized management of VMs and containers
across multiple physical hosts.

## Key Features

- **Multi-engine**: QEMU/KVM, Firecracker, Docker/Podman (Linux); Bhyve, Jail (FreeBSD)
- **Multi-host fleet**: up to 50 physical hosts, 1000 VM instances
- **Native FS support**: ZFS snapshots, Btrfs subvolumes, raw file copies
- **Environments**: group related VMs with lifecycle control and auto-expiry (default 6h)
- **Web dashboard**: built-in browser UI served by the controller
- **Simple deployment**: three binaries, SQLite persistence, idempotent deploy scripts
- **HTTP REST API**: JSON API for automation and CI/CD

## Architecture

```
┌──────────┐             ┌──────────────┐             ┌───────────┐
│  tt CLI  ├──── HTTP ──►│   tt-ctl     ├──── HTTP ──►│ tt-agent  │ × N
└──────────┘             │ (controller) │             │ (per-host)│
┌──────────┐             │ + Web UI     │             └─────┬─────┘
│ Browser  ├──── HTTP ──►└──────┬───────┘                   │
└──────────┘                    │                    VM engines + storage
                           SQLite DB
```

| Binary | Role |
|--------|------|
| **tt** | CLI client |
| **tt-ctl** | Central controller: scheduling, state, web UI |
| **tt-agent** | Host agent: VM lifecycle, images, networking |

## Quick Start

### 1. Build and install

```bash
make release

# Or use the idempotent deploy script (requires root):
make deploy          # both agent + controller on this host
make deploy-agent    # agent only
make deploy-ctl      # controller only

# Or distributed deploy via SSH:
cp tools/deploy.toml.example deploy.toml
# edit deploy.toml with your fleet IPs...
make deploy-dist
```

Directory layout after deploy:
```
/opt/ttstack/bin/          # binaries (tt, tt-ctl, tt-agent)
/home/ttstack/             # runtime data (dedicated ttstack user)
  ├── images/              # base VM/container images
  ├── runtime/             # transient VM image clones
  ├── data/                # agent SQLite database
  ├── ctl/                 # controller SQLite database
  └── run/                 # PID files, sockets
```

### 2. Register hosts and create environments

```bash
tt config 10.0.0.1:9200             # point CLI to controller
tt host add 10.0.0.2:9100           # register host

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

### 3. Web dashboard

Open `http://<controller-addr>:9200` in a browser.

## CLI Reference

```
tt config <addr>                    Set controller address
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
| `--engine <type>` | qemu, firecracker, docker (Linux); bhyve, jail (FreeBSD) | qemu |
| `--cpu <N>` | vCPUs per VM | 2 |
| `--mem <MiB>` | Memory per VM in MiB | 1024 (1 GiB) |
| `--disk <MiB>` | Disk per VM in MiB | 40960 (40 GiB) |
| `--dup <N>` | Replicas per image | 1 |
| `-p, --port <PORT>` | Guest port to expose (repeatable) | — |
| `--lifetime <SEC>` | Auto-expiry in seconds (0 = 6h default) | 21600 |
| `--deny-outgoing` | Block outbound traffic | false |
| `--owner <USER>` | Owner label | `$USER` |

## Platform Support

| Platform | Engines | Networking |
|----------|---------|------------|
| **Linux** | QEMU/KVM, Firecracker, Docker/Podman | nftables NAT |
| **FreeBSD** | Bhyve, Jail | PF NAT |

Compile-time enforced — the project will not build on other platforms.

**Note**: Bhyve does not support in-place pause/resume. `env stop` + `env start`
on FreeBSD with Bhyve requires re-creating the VM.

## Storage Backends

| Backend | How it works | When to use |
|---------|-------------|-------------|
| **zfs** | ZFS snapshot + clone | Production (instant, space-efficient) |
| **btrfs** | Btrfs subvolume snapshot | Production (instant, space-efficient) |
| **raw** | `cp --reflink=auto` | Development, any filesystem |

## Agent Configuration

All resources in **MiB**. Set to 0 for auto-detection.

```
tt-agent [OPTIONS]

  --listen <ADDR>         Listen address              [0.0.0.0:9100]
  --image-dir <PATH>      Base image directory         [/home/ttstack/images]
  --runtime-dir <PATH>    Runtime clone directory      [/home/ttstack/runtime]
  --data-dir <PATH>       Database directory            [/home/ttstack/data]
  --storage <TYPE>        zfs | btrfs | raw            [raw]
  --cpu-total <N>         CPU cores (0=auto)           [0]
  --mem-total <MiB>       Memory in MiB (0=auto)       [0]
  --disk-total <MiB>      Disk in MiB                  [204800 (~200 GiB)]
  --host-id <ID>          Host ID (auto-generated)
```

## Controller Configuration

```
tt-ctl [OPTIONS]

  --listen <ADDR>       Listen address              [0.0.0.0:9200]
  --data-dir <PATH>     Database directory            [/home/ttstack/ctl]
```

## REST API

### Controller endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/` | Web dashboard |
| POST | `/api/hosts` | Register a host |
| GET | `/api/hosts` | List hosts |
| GET | `/api/hosts/{id}` | Host details |
| DELETE | `/api/hosts/{id}` | Remove host |
| POST | `/api/envs` | Create environment |
| GET | `/api/envs` | List environments |
| GET | `/api/envs/{id}` | Environment + VM details |
| DELETE | `/api/envs/{id}` | Destroy environment |
| POST | `/api/envs/{id}/stop` | Stop environment |
| POST | `/api/envs/{id}/start` | Start environment |
| GET | `/api/vms/{id}` | Single VM details |
| GET | `/api/images` | List images across fleet |
| GET | `/api/status` | Fleet-wide resource status |

### Agent endpoints

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

## Deployment

### Prerequisites

**Linux agents**: nftables, `tun`/`vhost_net`/`kvm_intel` kernel modules, `socat` (for QEMU monitor)

**FreeBSD agents**: PF enabled

### Distributed deploy

Edit `deploy.toml` (see `tools/deploy.toml.example`):

```toml
[controller]
host = "10.0.0.1"

[[agents]]
host = "10.0.0.2"

[[agents]]
host = "10.0.0.3"
storage = "zfs"
image_dir = "tank/ttstack/images"
cpu_total = 32
mem_total = 65536     # 64 GiB
disk_total = "1000G"  # ~1 TiB
```

Then: `make deploy-dist`

The deploy script is idempotent — safe to re-run for upgrades.
Schema migrations run automatically on startup.

## Project Structure

```
TTstack/
├── Cargo.toml              Workspace
├── Makefile                Build + deploy targets
├── tools/
│   ├── deploy.sh           Idempotent deploy script (local + SSH)
│   └── deploy.toml.example Fleet configuration template
└── crates/
    ├── core/               Shared library (ttcore)
    │   └── src/
    │       ├── model.rs    Data models, validation, constants
    │       ├── api.rs      Request/response types
    │       ├── net.rs      Platform-specific networking (nftables/PF)
    │       ├── engine/     VmEngine trait + {qemu,firecracker,docker,bhyve,jail}
    │       └── storage/    ImageStore trait + {zfs,btrfs,raw}
    ├── agent/              Host agent (tt-agent)
    │   └── src/
    │       ├── main.rs     Entry point + graceful shutdown
    │       ├── config.rs   CLI config
    │       ├── handler.rs  HTTP handlers
    │       └── runtime.rs  VM lifecycle + SQLite + schema migration
    ├── ctl/                Controller (tt-ctl)
    │   └── src/
    │       ├── main.rs     Entry point + env expiry + graceful shutdown
    │       ├── handler.rs  HTTP handlers
    │       ├── scheduler.rs Best-fit VM placement
    │       ├── db.rs       SQLite state + schema migration
    │       └── web.rs      Embedded web dashboard
    └── cli/                CLI client (tt)
        └── src/
            ├── main.rs     Command definitions
            └── client.rs   HTTP client + config file
```

## License

MIT
