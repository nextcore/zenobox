# 📦 Zenobox

<p align="center">
  <img src="https://img.shields.io/badge/language-Rust_2024-orange?logo=rust&style=for-the-badge" alt="Rust 2024">
  <img src="https://img.shields.io/badge/license-Apache--2.0-blue?style=for-the-badge" alt="Apache License">
  <img src="https://img.shields.io/badge/OCI-Compliant-brightgreen?style=for-the-badge" alt="OCI Compliant">
  <img src="https://img.shields.io/badge/Docker_API-v1.40--v1.47-cyan?logo=docker&style=for-the-badge" alt="Docker API">
  <img src="https://img.shields.io/badge/1Panel-100%25_Compatible-brightgreen?style=for-the-badge" alt="1Panel Compatible">
</p>

<p align="center">
  <b>Ultra-lightweight, standalone OCI Container Runtime & Docker Engine Alternative in Rust.</b><br>
  <i>Runs standard containers & Docker Compose with ~6MB RAM footprint—daemonless by default or 1Panel API compatible.</i>
</p>

---

## 🔥 Why Zenobox?

- **⚡ Lightweight & Blazing Fast**: Single native Rust binary (~7.4MB static MUSL build, **~6MB RAM daemon footprint**). Uses **95% less memory** than Docker Engine + containerd.
- **🔌 100% Docker Engine API & Unix Socket Compatible**: Direct `/var/run/docker.sock` & TCP `2375` REST API supporting Docker API `v1.40` through `v1.47` (including `/_ping`, `/version`, `/info`, `/containers/*`, `/images/*`).
- **🖥️ 1Panel & Portainer Ready**: Native auto-detection and registration as Systemd (`docker.service`) or OpenRC service. Fully compatible with control panels like **1Panel**, **Portainer**, and VSCode Docker extension.
- **💻 Dual Mode Flexibility (Daemonless or Service)**: Run standalone daemonless CLI commands directly, or run background daemon service.
- **🖥️ Real-time Interactive PTY Terminal**: Full WebSocket terminal exec (`/attach/ws`, `/exec/{id}/ws`) powered by `portable-pty` with dynamic window resize support.
- **📊 Live Cgroup Stats Streaming**: Streaming CPU & Memory usage metrics directly from Linux `cgroups` (v1 & v2).
- **🚀 Embedded Docker Compose Engine**: Native YAML orchestrator supporting `.env` interpolation (`${VAR:-default}`).
- **🔒 Production Ready & Rock Solid**: Non-blocking state persistence, 10MB auto-rotating container logs, and boot-time network auto-recovery.

---

## 🛠️ Quick Start

### 1-Click Automated Installation (Linux)
```bash
curl -fsSL https://raw.githubusercontent.com/nextcore/zenobox/main/install.sh | sudo bash
```
*Auto-detects Linux distribution (Ubuntu, Debian, LMDE, Alpine, RHEL/CentOS, Arch), installs static MUSL binary, configures `/var/run/docker.sock` and `docker.service` Systemd/OpenRC background daemon.*

### Uninstallation
```bash
curl -fsSL https://raw.githubusercontent.com/nextcore/zenobox/main/uninstall.sh | sudo bash
```
*To also purge image cache and container data, add `--purge`:* `sudo ./uninstall.sh --purge`

### Build & Release from Source
```bash
git clone https://github.com/nextcore/zenobox.git
cd zenobox

# Build standard release
cargo build --release

# Or build static MUSL release for Alpine / Any Linux with cache clean options:
./release_alpine.sh --clean --clean-dist
```

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

# 6. Start API Daemon manually (Unix Socket + TCP 2375)
zenobox daemon --port 2375 --socket /var/run/docker.sock
```

---

## 🌐 API & Control Panel Integration (1Panel / Portainer)

Zenobox provides a drop-in replacement for the Docker Engine API over both **Unix Domain Socket (`/var/run/docker.sock`)** and **HTTP TCP (`:2375`)**:

| Feature | Docker Engine API Endpoint | Native REST Endpoint |
| :--- | :--- | :--- |
| **Ping & Version** | `GET /v1.41/_ping`, `GET /v1.41/version` | `GET /_ping`, `GET /version` |
| **Engine Info** | `GET /v1.41/info` | `GET /info` |
| **Container Lifecycle** | `GET /v1.41/containers/json`, `POST /create` | `GET/POST /api/v1/containers` |
| **Live Stats Stream** | `GET /v1.41/containers/{id}/stats` | `GET /api/v1/containers/{id}/stats` |
| **Interactive Terminal (WebSocket)** | `GET /v1.41/containers/{id}/attach/ws` | `GET /api/v1/containers/{id}/terminal` |
| **Exec Session** | `POST /v1.41/containers/{id}/exec`, `/exec/{id}/start` | `POST /api/v1/exec` |

---

## 🔗 Transparent Docker & Docker Compose Replacement

Use Zenobox seamlessly in place of `docker` and `docker-compose`:

```bash
# Set shell aliases
alias docker="zenobox"
alias docker-compose="zenobox compose"
```

Or create global system symlinks (done automatically by `install.sh`):
```bash
sudo ln -sf /opt/zenobox/bin/zenobox /usr/local/bin/docker
sudo ln -sf /opt/zenobox/bin/zenobox /usr/local/bin/docker-compose
```

---

## ⚖️ Zenobox vs Docker Engine (`dockerd`)

| Capability | Docker Engine (`dockerd`) | Zenobox Runtime |
| :--- | :--- | :--- |
| **Memory Footprint (Idle Daemon)** | Heavy (~150MB - 300MB RAM) | **Ultra-Lightweight (~6MB RAM)** |
| **Standalone Daemonless Mode** | ❌ Requires running daemon | ✅ **Supported (Daemonless by default)** |
| **Control Panel Support (1Panel, Portainer)** | ✅ Supported | ✅ **100% Supported (`/var/run/docker.sock`)** |
| **Single-Node Containers & Compose** | ✅ Supported | ✅ **100% Supported** |
| **Bridge Networking & Ports** | ✅ `docker0` | ✅ `zenobr0` (`veth` + `iptables` NAT) |
| **Interactive PTY & Live Stats** | ✅ Supported | ✅ **Supported (WebSocket + Cgroups)** |
| **Docker API Compatibility** | ✅ Native | ✅ **v1.40 - v1.47 Full Support** |
| **Image Building (`docker build`)** | ✅ Built-in BuildKit | ❌ **Not Supported** (Pull pre-built images from registry) |
| **Multi-Host Clustering (`docker swarm`)** | ✅ Supported | ❌ **Not Supported** (Single-Node VPS focus) |

---

## ⚠️ Scope & Intentional Omissions

Zenobox is engineered specifically as a **lightweight single-node OCI container runtime**. To maintain its minimal footprint (~6MB RAM) and zero-daemon simplicity, the following components are intentionally omitted:

1. **❌ Image Building (`docker build` / `buildkit`)**
   - **Why**: Image building requires complex layer overlay caching and compiler tooling that bloats runtime binaries.
   - **Workflow**: Pull pre-compiled OCI images directly from registries (Docker Hub, GHCR, Quay) or use CI/CD pipelines (GitHub Actions, GitLab CI) to build images.
2. **❌ Multi-Host Clustering (`docker swarm`)**
   - **Why**: Zenobox targets single-node Linux servers, VPS instances, edge devices, and panel backends.
   - **Workflow**: For single-host multi-container stacks, use `zenobox compose` or native REST API.

---

## 📄 License

Licensed under [Apache License 2.0](./LICENSE).
