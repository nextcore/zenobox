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
  <i>Runs standard containers & Docker Compose with ~3.8MB RAM footprint—daemonless by default or 1Panel API compatible.</i>
</p>

---

## 🔥 Why Zenobox?

- **⚡ Lightweight & Blazing Fast**: Single native Rust binary (~15MB static MUSL build, **~3.8MB RAM daemon footprint**). Uses **~95% - 98% less memory** than Docker Engine + containerd.
- **🔌 100% Docker Engine API & Unix Socket Compatible**: Direct `/var/run/docker.sock` & TCP `2375` REST API supporting Docker API `v1.40` through `v1.47`.
- **🖥️ 1Panel Tested & Ready**: Native auto-detection as Systemd (`docker.service`) or OpenRC. Fully tested with **1Panel AppStore & Web UI Terminal Exec**.
- **💻 Dual Mode Flexibility**: Run standalone daemonless CLI commands directly, or run background daemon service.
- **🖥️ Real-time Interactive PTY Terminal**: Full WebSocket & CLI terminal exec (`/exec/{id}/ws`, `docker exec -it`) with dynamic window resize support.
- **📊 Live Cgroup Stats Streaming**: Streaming CPU & Memory metrics directly from Linux `cgroups` (v1 & v2).
- **🚀 Embedded Docker Compose Engine**: Native YAML orchestrator supporting `.env` interpolation (`${VAR:-default}`).

---

## 🛠️ Quick Start

### 1-Click Automated Installation (Linux)
```bash
curl -fsSL https://raw.githubusercontent.com/nextcore/zenobox/main/install.sh | sudo bash
```
*Auto-detects Linux distribution, installs static MUSL binary, configures `/var/run/docker.sock` and Systemd/OpenRC background daemon.*

### Uninstallation
```bash
curl -fsSL https://raw.githubusercontent.com/nextcore/zenobox/main/uninstall.sh | sudo bash
```
*To also purge image cache and container data:* `sudo ./uninstall.sh --purge`

### Build from Source
```bash
git clone https://github.com/nextcore/zenobox.git && cd zenobox
./build_alpine.sh --clean
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

# 6. Transparent Docker Symlinks (automatic via install.sh)
alias docker="zenobox"
alias docker-compose="zenobox compose"
```

---

## ⚖️ Zenobox vs Docker Engine (`dockerd`)

| Capability | Docker Engine (`dockerd`) | Zenobox Runtime |
| :--- | :--- | :--- |
| **Memory Footprint (Idle Daemon)** | Heavy (~150MB - 300MB RAM) | **Ultra-Lightweight (~3.8MB - 6.8MB RAM)** |
| **Standalone Daemonless Mode** | ❌ Requires running daemon | ✅ **Supported (Daemonless by default)** |
| **Control Panel Support (1Panel)** | ✅ Supported | ✅ **100% 1Panel Tested & Compatible** |
| **Single-Node Containers & Compose** | ✅ Supported | ✅ **100% Supported** |
| **Interactive PTY & Live Stats** | ✅ Supported | ✅ **Supported (WebSocket + Cgroups)** |
| **Docker API Compatibility** | ✅ Native | ✅ **v1.40 - v1.47 Support** |
| **Image Building (`docker build`)** | ✅ Built-in BuildKit | ❌ **Not Supported** (Pull pre-built images from registry) |
| **Multi-Host Clustering (`docker swarm`)** | ✅ Supported | ❌ **Not Supported** (Single-Node VPS focus) |

---

## ⚡ Memory Footprint

Zenobox is designed to run efficiently even on low-spec VPS instances ($2/mo VPS with 512MB RAM):

| Process | Memory Footprint (RSS) | Memory Reduction | CPU Overhead |
| :--- | :--- | :--- | :--- |
| **Zenobox Daemon (`zenobox daemon`)** | **~3.8 MB - 6.8 MB RAM** *(Peak ~10M)* | **~95% - 98% Less Memory** | **~0.1% CPU (Instant <1ms response)** |
| **Docker Engine (`dockerd` + `containerd`)** | **~150 MB - 250 MB RAM** | Standard Baseline | ~2% - 5% CPU |

---

## 📄 License

Licensed under [Apache License 2.0](./LICENSE).
