#!/usr/bin/env bash
#
# TTstack deployment script — idempotent, supports local & distributed
#
# Local deployment:
#   ./tools/deploy.sh agent        Deploy tt-agent locally
#   ./tools/deploy.sh ctl          Deploy tt-ctl locally
#   ./tools/deploy.sh all          Deploy both locally
#
# Distributed deployment (via SSH, reads deploy.toml):
#   ./tools/deploy.sh -c deploy.toml
#
# Environment overrides (local mode only):
#   PREFIX  TTUSER  TTHOME  LISTEN_AGENT  LISTEN_CTL

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

PREFIX="${PREFIX:-/opt/ttstack}"
TTUSER="${TTUSER:-ttstack}"
TTHOME="${TTHOME:-/home/$TTUSER}"

BIN_DIR="$PREFIX/bin"
RELEASE_DIR="${RELEASE_DIR:-$PROJECT_DIR/target/release}"

# ── Helpers ──────────────────────────────────────────────────────────

log()  { echo "[deploy] $*"; }
err()  { echo "[deploy] ERROR: $*" >&2; exit 1; }

need_root() {
    [ "$(id -u)" -eq 0 ] || err "must be run as root (or via sudo)"
}

# Convert disk_total value: "200G" → 204800 (MiB), plain number passes through
parse_disk() {
    local val="$1"
    val="${val%\"}" ; val="${val#\"}"
    if [[ "$val" =~ ^([0-9]+)[gG]$ ]]; then
        echo $(( ${BASH_REMATCH[1]} * 1024 ))
    else
        echo "$val"
    fi
}

# ── Idempotent local setup ───────────────────────────────────────────

ensure_user() {
    if id "$TTUSER" &>/dev/null; then
        log "user '$TTUSER' exists"
    else
        log "creating user '$TTUSER'"
        useradd -r -m -d "$TTHOME" -s /bin/bash "$TTUSER"
    fi
}

ensure_dirs() {
    mkdir -p "$BIN_DIR" \
        "$TTHOME/images" "$TTHOME/runtime" \
        "$TTHOME/data"   "$TTHOME/ctl" "$TTHOME/run"
    chown -R "$TTUSER:$TTUSER" "$TTHOME"
}

install_bin() {
    local bin="$1"
    local src="$RELEASE_DIR/$bin"
    [ -f "$src" ] || err "$src not found (run 'make release' first)"
    install -m 755 "$src" "$BIN_DIR/$bin"
    log "installed $BIN_DIR/$bin"
}

install_systemd_unit() {
    local name="$1" exec_start="$2" run_as_root="${3:-false}"
    local user_line="User=${TTUSER}"
    local group_line="Group=${TTUSER}"
    if [ "$run_as_root" = "true" ]; then
        user_line="# Runs as root (needs NET_ADMIN for bridge/TAP/nftables)"
        group_line=""
    fi
    cat > "/etc/systemd/system/${name}.service" <<EOF
[Unit]
Description=TTstack ${name}
After=network.target

[Service]
Type=simple
${user_line}
${group_line}
ExecStart=${exec_start}
Restart=on-failure
RestartSec=5
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
EOF
    systemctl daemon-reload
    systemctl enable "$name"
    log "unit $name installed"
}

restart_service() {
    local name="$1"
    if systemctl is-active --quiet "$name"; then
        systemctl restart "$name"
    else
        systemctl start "$name"
    fi
    log "$name is $(systemctl is-active "$name")"
}

# ── Local deploy functions ───────────────────────────────────────────

local_deploy_agent() {
    # Args: [listen] [storage] [image_dir] [runtime_dir]
    #       [cpu_total] [mem_total] [disk_total] [host_id]
    local listen="${1:-0.0.0.0:9100}"
    local storage="${2:-raw}"
    local image_dir="${3:-$TTHOME/images}"
    local runtime_dir="${4:-$TTHOME/runtime}"
    local cpu="${5:-0}" mem="${6:-0}" disk="$(parse_disk "${7:-200G}")"
    local host_id="${8:-}"

    log "deploying tt-agent (listen=$listen storage=$storage)"
    install_bin "tt-agent"

    local cmd="$BIN_DIR/tt-agent"
    cmd="$cmd --listen $listen"
    cmd="$cmd --image-dir $image_dir"
    cmd="$cmd --runtime-dir $runtime_dir"
    cmd="$cmd --data-dir $TTHOME/data"
    cmd="$cmd --storage $storage"
    cmd="$cmd --cpu-total $cpu"
    cmd="$cmd --mem-total $mem"
    cmd="$cmd --disk-total $disk"
    [ -n "$host_id" ] && cmd="$cmd --host-id $host_id"

    install_systemd_unit "tt-agent" "$cmd" "true"
    restart_service "tt-agent"
}

local_deploy_ctl() {
    local listen="${1:-0.0.0.0:9200}"
    local data_dir="${2:-$TTHOME/ctl}"

    log "deploying tt-ctl (listen=$listen)"
    install_bin "tt-ctl"
    install_bin "tt"
    install_systemd_unit "tt-ctl" \
        "$BIN_DIR/tt-ctl --listen $listen --data-dir $data_dir"
    restart_service "tt-ctl"
}

# ── Minimal TOML parser ──────────────────────────────────────────────
# Produces shell variables: CFG_<section>_<key>=<value>

parse_toml() {
    local file="$1"
    local section="" agent_idx=-1

    while IFS= read -r line; do
        line="${line%%#*}"
        line="$(echo "$line" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
        [ -z "$line" ] && continue

        if [[ "$line" == "[[agents]]" ]]; then
            agent_idx=$((agent_idx + 1))
            section="agent_${agent_idx}"
            continue
        elif [[ "$line" =~ ^\[([a-zA-Z_]+)\]$ ]]; then
            section="${BASH_REMATCH[1]}"
            continue
        fi

        if [[ "$line" =~ ^([a-zA-Z_]+)[[:space:]]*=[[:space:]]*(.+)$ ]]; then
            local key="${BASH_REMATCH[1]}"
            local val="${BASH_REMATCH[2]}"
            val="${val%\"}" ; val="${val#\"}"
            printf 'CFG_%s_%s=%s\n' "$section" "$key" "$val"
        fi
    done < "$file"

    printf 'CFG_agent_count=%s\n' "$((agent_idx + 1))"
}

# ── SSH helpers ──────────────────────────────────────────────────────

remote_exec() {
    local ssh_user="$1" host="$2" port="$3"
    shift 3
    ssh -o StrictHostKeyChecking=no -o ConnectTimeout=10 \
        -p "$port" "${ssh_user}@${host}" "$@"
}

remote_copy() {
    local ssh_user="$1" host="$2" port="$3" src="$4" dst="$5"
    scp -o StrictHostKeyChecking=no -o ConnectTimeout=10 \
        -P "$port" "$src" "${ssh_user}@${host}:${dst}"
}

# ── Distributed deploy ───────────────────────────────────────────────

distributed_deploy() {
    local config_file="$1"
    [ -f "$config_file" ] || err "config not found: $config_file"

    log "parsing $config_file"
    eval "$(parse_toml "$config_file")"

    local prefix="${CFG_general_prefix:-/opt/ttstack}"
    local user="${CFG_general_user:-ttstack}"
    local home="/home/$user"
    local release="${CFG_general_release_dir:-./target/release}"

    for bin in tt tt-ctl tt-agent; do
        [ -f "$release/$bin" ] || err "$release/$bin not found (run 'make release' first)"
    done

    # Upload binaries + deploy script, then run remotely
    deploy_remote() {
        local ssh_user="$1" host="$2" port="$3"
        shift 3
        local role="$1"; shift
        # remaining args: passed to deploy.sh on remote

        log "=== $host ($role) ==="

        local tmp="/tmp/ttstack-deploy-$$"
        remote_exec "$ssh_user" "$host" "$port" "mkdir -p $tmp"

        case "$role" in
            agent)
                remote_copy "$ssh_user" "$host" "$port" "$release/tt-agent" "$tmp/tt-agent"
                ;;
            ctl)
                remote_copy "$ssh_user" "$host" "$port" "$release/tt-ctl" "$tmp/tt-ctl"
                remote_copy "$ssh_user" "$host" "$port" "$release/tt" "$tmp/tt"
                ;;
        esac

        remote_copy "$ssh_user" "$host" "$port" "$SCRIPT_DIR/deploy.sh" "$tmp/deploy.sh"

        local env="PREFIX=$prefix TTUSER=$user TTHOME=$home RELEASE_DIR=$tmp"
        remote_exec "$ssh_user" "$host" "$port" "$env bash $tmp/deploy.sh $*"
        remote_exec "$ssh_user" "$host" "$port" "rm -rf $tmp"
        log "=== $host done ==="
    }

    # Deploy controller
    if [ -n "${CFG_controller_host:-}" ]; then
        local ch="${CFG_controller_host}"
        local cp="${CFG_controller_ssh_port:-22}"
        local cu="${CFG_controller_ssh_user:-root}"
        local cl="${CFG_controller_listen:-0.0.0.0:9200}"
        local cd="${CFG_controller_data_dir:-$home/ctl}"

        deploy_remote "$cu" "$ch" "$cp" \
            "ctl" "ctl" "$cl" "$cd"
    fi

    # Deploy agents
    local i=0
    while [ $i -lt "${CFG_agent_count:-0}" ]; do
        # Read all config keys for this agent (with defaults)
        local _h="CFG_agent_${i}_host"
        local _sp="CFG_agent_${i}_ssh_port"
        local _su="CFG_agent_${i}_ssh_user"
        local _li="CFG_agent_${i}_listen"
        local _st="CFG_agent_${i}_storage"
        local _id="CFG_agent_${i}_image_dir"
        local _rd="CFG_agent_${i}_runtime_dir"
        local _cpu="CFG_agent_${i}_cpu_total"
        local _mem="CFG_agent_${i}_mem_total"
        local _dk="CFG_agent_${i}_disk_total"
        local _hid="CFG_agent_${i}_host_id"

        local ahost="${!_h:-}"
        [ -n "$ahost" ] || { i=$((i + 1)); continue; }

        deploy_remote \
            "${!_su:-root}" "$ahost" "${!_sp:-22}" \
            "agent" "agent" \
            "${!_li:-0.0.0.0:9100}" \
            "${!_st:-raw}" \
            "${!_id:-$home/images}" \
            "${!_rd:-$home/runtime}" \
            "${!_cpu:-0}" \
            "${!_mem:-0}" \
            "${!_dk:-200G}" \
            "${!_hid:-}"

        i=$((i + 1))
    done

    log "distributed deployment complete"
}

# ── Main ─────────────────────────────────────────────────────────────

main() {
    # Distributed mode
    if [ "${1:-}" = "-c" ]; then
        [ -n "${2:-}" ] || err "usage: $0 -c <deploy.toml>"
        distributed_deploy "$2"
        return
    fi

    local target="${1:-}"
    shift || true

    case "$target" in
        agent)
            need_root; ensure_user; ensure_dirs
            local_deploy_agent "$@"
            ;;
        ctl)
            need_root; ensure_user; ensure_dirs
            local_deploy_ctl "$@"
            ;;
        all)
            need_root; ensure_user; ensure_dirs
            local_deploy_agent
            local_deploy_ctl
            ;;
        *)
            cat <<USAGE
TTstack deployment — idempotent

Local (requires root):
  $0 agent              Deploy tt-agent on this host
  $0 ctl                Deploy tt-ctl + web UI on this host
  $0 all                Deploy both on this host

Distributed (via SSH):
  $0 -c deploy.toml     Deploy to fleet defined in config

See tools/deploy.toml.example for configuration format.
USAGE
            exit 1
            ;;
    esac
}

main "$@"
