# REST API Reference

All `/api/*` endpoints require `Authorization: Bearer <api-key>` when the
controller is started with `--api-key`. The web dashboard (`/`) is always open.

## Controller Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/` | Web dashboard (no auth required) |
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

## Agent Endpoints

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

## Examples

### Register a host

```bash
curl -X POST http://controller:9200/api/hosts \
  -H "Authorization: Bearer <key>" \
  -H "Content-Type: application/json" \
  -d '{"addr": "10.0.0.2:9100"}'
```

### Create an environment

```bash
curl -X POST http://controller:9200/api/envs \
  -H "Authorization: Bearer <key>" \
  -H "Content-Type: application/json" \
  -d '{
    "id": "my-env",
    "owner": "alice",
    "ssh_keys": ["ssh-ed25519 AAAA... alice@laptop"],
    "vms": [
      {
        "image": "alpine-cloud",
        "engine": "qemu",
        "cpu": 2,
        "mem": 2048,
        "disk": 40960,
        "ports": [80],
        "deny_outgoing": false
      }
    ],
    "lifetime": 21600
  }'
```

### Fleet status

```bash
curl -H "Authorization: Bearer <key>" http://controller:9200/api/status
```

## Request / Response Reference

### CreateEnvReq (POST `/api/envs`)

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | yes | Environment name |
| `owner` | string | no | Owner label |
| `ssh_keys` | string[] | yes | SSH public keys injected into all VMs (cloud-init `authorized_keys`) |
| `vms` | VmSpec[] | yes | List of VM specifications |
| `lifetime` | integer | no | Auto-expiry in seconds (default: 21600 = 6h) |

### VmSpec (element of `vms` array)

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `image` | string | yes | Base image name |
| `engine` | string | no | `qemu`, `firecracker`, `docker`, `bhyve`, `jail` (default: `qemu`) |
| `cpu` | integer | no | vCPUs (default: 2) |
| `mem` | integer | no | Memory in MiB (default: 1024) |
| `disk` | integer | no | Disk in MiB (default: 40960) |
| `ports` | integer[] | no | Guest ports to expose; port 22 is always auto-included |
| `deny_outgoing` | boolean | no | Block outbound traffic (default: false) |

### Storage field (agent `/api/info`)

The `storage` field in host info reports the backend type:
- `"file"` — plain qcow2 files, filesystem-agnostic (aliases: `"raw"`)
- `"zvol"` — ZFS zvol raw block devices (aliases: `"zfs"`)
