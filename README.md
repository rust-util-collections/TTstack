# TTstack — Lightweight Private Cloud

[![CI](https://github.com/rust-util-collections/TTstack/actions/workflows/ci.yml/badge.svg)](https://github.com/rust-util-collections/TTstack/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.86%2B-orange.svg)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/platform-linux%20%7C%20freebsd-green.svg)](#platform-support)

TTstack is a lightweight private cloud platform for mid-size teams and
individual developers. Centralized management of VMs and containers
across multiple physical hosts.

## Quick Start

```bash
make release
sudo tt deploy all                          # deploy agent + controller
sudo tt image create all --engine docker    # generate Docker images
sudo tt image create alpine-cloud           # generate QEMU cloud image (SSH-ready)

tt config <controller-ip>:9200 --api-key <api-key> # key printed by deploy
tt host add <agent-ip>:9100                 # register a host

tt env create demo --image alpine-cloud --engine qemu \
  --ssh-key ~/.ssh/id_ed25519.pub
tt env show demo                            # see port mappings
ssh root@<host-ip> -p <mapped-port>         # key-based auth
```

## VM Access

| Engine | How to access |
|--------|--------------|
| **QEMU** cloud images | `ssh root@<host> -p <mapped-port>` (SSH key injected via cloud-init) |
| **QEMU** custom images | SSH via port forwarding (your own key setup) |
| **Docker** | SSH (if sshd in image) or `docker exec` from host |
| **Firecracker** | serial console only |
| **Bhyve** (FreeBSD) | SSH via port forwarding |

QEMU cloud images auto-configure via **cloud-init**: SSH public keys,
networking — all set on first boot. See [docs/guest-images.md](docs/guest-images.md).

## Security

All `/api/*` endpoints require a Bearer token when `--api-key` is set
(auto-generated on deploy). The web dashboard (`/`) remains open.

```bash
# Set in deploy.toml:
[general]
api_key = "your-secret-key"

# Or configure CLI directly:
tt config <addr> --api-key <api-key>

# Or via environment:
export TT_API_KEY=your-secret-key
```

## Built-in Images

12 ready-to-use recipes — deploy and start creating VMs immediately:

| Recipe | Engine | Description |
|--------|--------|-------------|
| `alpine` `debian` `ubuntu` `rockylinux` | Docker | Base OS containers |
| `nginx` `redis` `postgres` | Docker | Popular services |
| `fc-alpine` | Firecracker | Alpine microVM (~50MB) |
| `alpine-cloud` `debian-cloud` `ubuntu-cloud` | QEMU | SSH-ready cloud images |
| `freebsd-base` | Jail | FreeBSD 14.3 base |

```bash
tt image recipes                            # list all
sudo tt image create all --engine docker    # all Docker images
sudo tt image create alpine-cloud           # one QEMU cloud image
sudo tt image create all                    # everything for this platform
```

See [docs/guest-images.md](docs/guest-images.md) for custom image creation.

## Key Features

- **Multi-engine**: QEMU/KVM, Firecracker, Docker/Podman (Linux); Bhyve, Jail (FreeBSD)
- **Multi-host fleet**: up to 50 hosts, 1000 VM instances, best-fit scheduling
- **Environments**: group VMs with lifecycle control and auto-expiry (default 6h)
- **Storage backends**: ZFS zvol (instant clone), plain qcow2 file copies
- **SSH key injection**: provide public keys at create time; port 22 auto-included
- **Web dashboard**: built-in monitoring UI at `http://<controller>:9200`
- **Simple deploy**: three binaries, SQLite, one command (`tt deploy all`)

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

## CLI Reference

```
tt config <addr> [--api-key <api-key>]     Set controller address and API key
tt status                           Fleet-wide status

tt host add/list/show/remove        Manage hosts
tt env create/list/show/delete      Manage environments
tt env stop/start <name>            Lifecycle control

tt image list/recipes/create        Manage images
tt deploy agent/ctl/all/dist        Deploy TTstack
```

### `env create` options

| Option | Description | Default |
|--------|-------------|---------|
| `-i, --image <name>` | Base image (repeatable) | *required* |
| `--engine <type>` | qemu, firecracker, docker, bhyve, jail | qemu |
| `--cpu <N>` | vCPUs per VM | 2 |
| `--mem <MiB>` | Memory per VM | 1024 |
| `--disk <MiB>` | Disk per VM | 40960 |
| `--dup <N>` | Replicas per image | 1 |
| `--ssh-key <FILE>` | SSH public key file (repeatable) | *required for VMs* |
| `-p, --port <PORT>` | Guest port to expose (repeatable) | — |
| `--lifetime <SEC>` | Auto-expiry (0 = 6h default) | 21600 |
| `--deny-outgoing` | Block outbound traffic | false |

## Platform Support

| Platform | Engines | Networking |
|----------|---------|------------|
| **Linux** | QEMU/KVM, Firecracker, Docker/Podman | nftables NAT |
| **FreeBSD** | Bhyve, Jail | PF NAT |

## Documentation

| Document | Contents |
|----------|----------|
| [docs/deployment.md](docs/deployment.md) | Full deployment guide, config reference, directory layout |
| [docs/guest-images.md](docs/guest-images.md) | Image formats, custom image creation, VM access details |
| [docs/rest-api.md](docs/rest-api.md) | REST API endpoints with curl examples |
| [docs/compatibility.md](docs/compatibility.md) | Platform test results and known issues |

## Project Structure

```
TTstack/
├── Cargo.toml              Workspace
├── Makefile                Build + deploy targets
├── tools/
│   └── deploy.toml.example Fleet configuration template
└── crates/
    ├── core/               Shared library (engines, storage, networking, models)
    ├── agent/              Host agent (tt-agent)
    ├── ctl/                Controller (tt-ctl)
    └── cli/                CLI client (tt)
```

## License

MIT
