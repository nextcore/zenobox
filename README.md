# 📦 Zenobox

[![Rust](https://img.shields.io/badge/language-Rust_2024-orange?logo=rust&style=flat-square)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache-blue?style=flat-square)](./LICENSE)
[![OCI Compliant](https://img.shields.io/badge/OCI-Compliant-brightgreen?style=flat-square)](#)

**Zenobox** is an ultra-lightweight, standalone **OCI Container Runtime & Docker Alternative** written in **Rust (2024 edition)**.

Engineered as a high-performance, drop-in replacement for `docker` and `docker-compose` with a minimal memory footprint (~15MB RAM), Zenobox executes OCI containers natively using an embedded `runc` runtime—**without requiring external daemons** such as `dockerd` or `containerd`.

---

## 🚀 Key Features

- **Zero External Daemons**: Runs containers standalone without `dockerd` or `containerd`. Single static binary execution.
- **Native OCI Image Puller**: Fetches and unpacks layer manifests directly from Docker Hub and OCI Registry V2 endpoints.
- **Embedded Docker Compose Engine**: Native Rust YAML orchestrator parsing `docker-compose.yml` without external dependencies.
- **Virtual Networking & Service Discovery**: Automatic Linux `veth` pair creation, bridge networking (`zenobr0`), iptables NAT rules, and container `/etc/hosts` DNS injection.
- **Volume Management**: Full support for named volumes and host bind mounts.
- **Dual Mode Architecture (CLI + Rust Library)**: Operates as a command-line tool (`zenobox`) or can be imported directly as a Rust library crate (`zenobox`) by web frameworks or control panels (e.g., ZenoPanel).

---

## 💻 CLI Usage (`zenobox`)

```bash
# 1. Pull OCI Image
zenobox pull alpine:latest

# 2. Run Container in background
zenobox run -d -p 8080:80 --name my-nginx nginx:alpine

# 3. List Running & Stopped Containers
zenobox ps -a

# 4. Stream Container Console Logs
zenobox logs my-nginx

# 5. Docker Compose Orchestration
zenobox compose up -d -f docker-compose.yml
zenobox compose down -f docker-compose.yml

# 6. Manage Images, Volumes, and Networks
zenobox image ls
zenobox volume ls
zenobox network ls

# 7. REST API & Docker API Daemon
zenobox daemon --port 2375
```

---

## 🌐 REST API & Docker Engine Socket API

Zenobox includes an embedded **Axum 0.8 / Tokio** REST API server listening by default on official Docker TCP Port **`2375`**.

### 1. Docker Engine API Compatibility (`/v1.41/...`)
Allows external tools (such as Portainer, 1Panel, VSCode Docker extension, or standard Docker HTTP clients) to connect directly:
```bash
# Docker API Ping
curl http://localhost:2375/_ping

# Docker List Containers
curl http://localhost:2375/v1.41/containers/json

# Docker Web Terminal Exec (Create & Start Exec Session)
curl -X POST http://localhost:2375/v1.41/containers/my-container/exec -H "Content-Type: application/json" -d '{"Cmd":["/bin/sh"],"Tty":true}'
curl -X POST http://localhost:2375/v1.41/exec/{exec_id}/start
```

### 2. Native REST API (`/api/v1/...`)
High-performance REST API for Web Dashboards & UI integrations:
- `GET /api/v1/containers` - List containers
- `POST /api/v1/containers` - Create and start container
- `POST /api/v1/compose/up` - Deploy docker-compose file
- `GET /api/v1/images` - List cached OCI images

---

## 🛠️ Build & Installation

### Automated 1-Click Installation
```bash
curl -fsSL https://raw.githubusercontent.com/nextcore/zenobox/main/install.sh | bash
```
*Installs Zenobox to `/opt/zenobox` and automatically sets up `/usr/local/bin/docker` & `/usr/local/bin/docker-compose` symlinks.*

The compiled binary will be located at `target/release/zenobox`.

### Alpine Linux / MUSL Static Release
To compile a fully static binary for Alpine Linux (MUSL):
```bash
./release_alpine.sh
```
*Creates static tarball `dist/zenobox-v0.1.0-x86_64-unknown-linux-musl.tar.gz` and SHA256 checksum.*

---

## 🔗 Using Zenobox as a `docker` Replacement (Alias & Symlink)

Zenobox is designed to be a transparent replacement for `docker` and `docker-compose`.

### Option A: Shell Aliases (`~/.bashrc` or `~/.zshrc`)
Add the following lines to your shell profile:
```bash
alias docker="zenobox"
alias docker-compose="zenobox compose"
```

### Option B: System Symlinks (Global Replacement)
Create symlinks in `/usr/local/bin` to allow scripts expecting `docker` or `docker-compose` to invoke Zenobox directly:
```bash
sudo ln -sf /path/to/target/release/zenobox /usr/local/bin/docker
sudo ln -sf /path/to/target/release/zenobox /usr/local/bin/docker-compose
```
*Note: When invoked as `docker-compose`, Zenobox automatically routes commands to `zenobox compose`.*

---

## ⚠️ Scope & Limitations (Zenobox vs Docker Engine)

Zenobox is engineered as an ultra-lightweight, high-performance **Docker Runtime replacement** tailored for single-node Linux servers, VPS instances, edge devices, and control panel backends (such as 1Panel, ZenoPanel, and Portainer).

> [!NOTE]
> **Ideal & Ready for Standard Docker Workloads**: Zenobox is **fully ready and optimized** for running single-node containerized web applications, microservices, databases (PostgreSQL, MySQL, Redis), web proxies (Nginx, Caddy, Traefik), and multi-container `docker-compose` stacks.

### Direct Comparison vs Docker Engine (`dockerd`)

| Feature / Capability | Docker Engine (`dockerd`) | Zenobox Runtime | Workload Suitability |
| :--- | :--- | :--- | :--- |
| **Footprint & Architecture** | Heavy Go Daemon + `containerd` (~150-300MB RAM) | Single Native Rust Binary (~7MB static MUSL, ~15MB RAM) | **Ideal for VPS & Edge**: Saves >90% RAM compared to Docker. |
| **Single-Node Containers & Compose** | ✅ Supported | ✅ Fully Supported | **100% Drop-in**: Runs standard `docker run` and `docker-compose.yml`. |
| **Bridge Networking & Port Forwarding** | ✅ Supported (`docker0`) | ✅ Fully Supported (`zenobr0`) | **Native Linux Bridge**: Full `veth` pairs, `iptables` NAT, & `-p` port mapping. |
| **Volume & Bind Mounts** | ✅ Supported | ✅ Fully Supported | **Native Mounts**: Supports named volumes (`-v volume_name:/path`) & host bind mounts (`-v /host:/path`). |
| **REST API Engine** | ✅ Docker Engine API | ✅ Docker v1.41 API + Native REST API | **100% Panel Compatible**: Connects to 1Panel, Portainer, & Web Dashboards. |
| **OCI Image Pulling** | ✅ OCI / Docker V2 | ✅ OCI / Docker V2 | **Full Compatibility**: Pulls from Docker Hub, GHCR, Quay.io. |
| **Multi-Host Clustering (Swarm)** | ✅ Supported (`docker swarm`) | ❌ Excluded (Single-Node Focus) | Designed for single-node VPS/server runtimes; multi-node Swarm plugins omitted. |
| **OS Compatibility** | Linux, macOS (Desktop), Windows | Native Linux / Alpine (WSL2 on Win/macOS) | Direct Linux Kernel OCI container execution. |

---

## 📄 License

Distributed under the [Apache 2.0 License](./LICENSE).
