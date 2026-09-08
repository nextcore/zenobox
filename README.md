# 📦 Zenobox

<p align="center">
  <img src="https://img.shields.io/badge/language-Rust_2024-orange?logo=rust&style=for-the-badge" alt="Rust 2024">
  <img src="https://img.shields.io/badge/license-Apache--2.0-blue?style=for-the-badge" alt="Apache License">
  <img src="https://img.shields.io/badge/OCI-Compliant-brightgreen?style=for-the-badge" alt="OCI Compliant">
  <img src="https://img.shields.io/badge/Docker_API-v1.40--v1.47-cyan?logo=docker&style=for-the-badge" alt="Docker API">
</p>

<p align="center">
  <b>Ultra-lightweight, standalone OCI Container Runtime & Docker Alternative in Rust.</b><br>
  <i>Runs standard containers & Docker Compose with ~15MB RAM footprint—zero external daemons required.</i>
</p>

---

## 🔥 Why Zenobox?

- **⚡ Lightweight & Blazing Fast**: Single native Rust binary (~7MB MUSL build, ~15MB RAM). Uses **90% less memory** than Docker Engine + containerd.
- **🔌 100% Docker API Compatible**: Drop-in REST API supporting Docker API `v1.40` through `v1.47`. Compatible with Portainer, 1Panel, VSCode, and HTTP clients.
- **🖥️ Real-time Interactive PTY Terminal**: Full WebSocket terminal exec (`/attach/ws`, `/terminal`) powered by `portable-pty` with dynamic resize support.
- **📊 Live Cgroup Stats Streaming**: Streaming CPU & Memory usage metrics directly from Linux `cgroups` (v1 & v2).
- **🚀 Embedded Docker Compose Engine**: Native YAML orchestrator supporting `.env` interpolation (`${VAR:-default}`).
- **🔒 Production Ready & Rock Solid**: Non-blocking `flock` state persistence, 10MB auto-rotating container logs, and boot-time network auto-recovery.
- **⚡ ZenoEngine Integration**: Embeds `.zl` script execution runtime (`zenoengine` v0.2.4).

---

## 🛠️ Quick Start

### 1-Click Automated Installation (Linux)
```bash
curl -fsSL https://raw.githubusercontent.com/nextcore/zenobox/main/install.sh | bash
```

### Build from Source
```bash
git clone https://github.com/nextcore/zenobox.git
cd zenobox
cargo build --release
```
*The compiled single binary is saved to `./target/release/zenobox`.*

---

## 💻 CLI Cheat Sheet

```bash
# 1. Pull an OCI Image (Docker Hub, GHCR, etc.)
zenobox pull alpine:latest

# 2. Run Container in background with Port Forwarding
zenobox run -d -p 8080:80 --name my-nginx nginx:alpine

# 3. List Containers
zenobox ps -a

# 4. Stream Logs & Exec Commands
zenobox logs my-nginx
zenobox exec my-nginx sh

# 5. Docker Compose Orchestration
zenobox compose up -d -f docker-compose.yml
zenobox compose down -f docker-compose.yml

# 6. Start API Daemon (Docker API on port 2375)
zenobox daemon --port 2375
```

---

## 🌐 API Features

Zenobox exposes an embedded **Axum / Tokio** REST server on port **`2375`**:

| Feature | Docker Engine API Endpoint | Native REST Endpoint |
| :--- | :--- | :--- |
| **Ping & Info** | `GET /v1.41/_ping`, `GET /v1.41/info` | `GET /_ping` |
| **Container Lifecycle** | `GET /v1.41/containers/json`, `POST /create` | `GET/POST /api/v1/containers` |
| **Live Stats Stream** | `GET /v1.41/containers/{id}/stats` | `GET /api/v1/containers/{id}/stats` |
| **Interactive Terminal (WebSocket)** | `GET /v1.41/containers/{id}/attach/ws` | `GET /api/v1/containers/{id}/terminal` |
| **Compose Up/Down** | — | `POST /api/v1/compose/up` |

---

## 🔗 Transparent Docker & Docker Compose Replacement

Use Zenobox seamlessly in place of `docker` and `docker-compose`:

```bash
# Set shell aliases
alias docker="zenobox"
alias docker-compose="zenobox compose"
```

Or create global system symlinks:
```bash
sudo ln -sf /usr/local/bin/zenobox /usr/local/bin/docker
sudo ln -sf /usr/local/bin/zenobox /usr/local/bin/docker-compose
```

---

## ⚖️ Zenobox vs Docker Engine (`dockerd`)

| Capability | Docker Engine (`dockerd`) | Zenobox Runtime |
| :--- | :--- | :--- |
| **Memory Footprint** | Heavy (~150MB - 300MB RAM) | **Ultra-Lightweight (~15MB RAM)** |
| **Daemons Required** | `dockerd` + `containerd` | **Zero (Standalone Static Binary)** |
| **Single-Node Containers & Compose** | ✅ Supported | ✅ **100% Supported** |
| **Bridge Networking & Ports** | ✅ `docker0` | ✅ `zenobr0` (`veth` + `iptables` NAT) |
| **Interactive PTY & Live Stats** | ✅ Supported | ✅ **Supported (WebSocket + Cgroups)** |
| **Docker API Compatibility** | ✅ Native | ✅ **v1.40 - v1.47 Full Support** |
| **Image Building (`docker build`)** | ✅ Built-in BuildKit | ❌ **Not Supported** (Pull pre-built images from registry) |
| **Multi-Host Clustering (`docker swarm`)** | ✅ Supported | ❌ **Not Supported** (Single-Node VPS focus) |

---

## ⚠️ Scope & Intentional Omissions

Zenobox is engineered specifically as a **lightweight single-node OCI container runtime**. To maintain its minimal footprint (~15MB RAM) and zero-daemon simplicity, the following components are intentionally omitted:

1. **❌ Image Building (`docker build` / `buildkit`)**
   - **Why**: Image building requires complex layer overlay caching and compiler tooling that bloats runtime binaries.
   - **Workflow**: Pull pre-compiled OCI images directly from registries (Docker Hub, GHCR, Quay) or use CI/CD pipelines (GitHub Actions, GitLab CI) to build images.
2. **❌ Multi-Host Clustering (`docker swarm`)**
   - **Why**: Zenobox targets single-node Linux servers, VPS instances, edge devices, and panel backends.
   - **Workflow**: For single-host multi-container stacks, use `zenobox compose` or native REST API.

---

## 📄 License

Licensed under [Apache License 2.0](./LICENSE).
