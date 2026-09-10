use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::process::Command;
use serde_json::json;

use crate::utils::{
    get_data_dir, container_dir, bundle_dir, rootfs_dir, state_file, log_path,
    runc_exec, run_privileged_status, get_runc_bin,
    save_container_state, load_container_state, rotate_log_file_if_needed,
    ContainerState, parse_image_ref
};
use crate::image::mount_overlayfs;
use crate::network::{configure_container_network, clean_container_network, sync_hosts_entries};

fn check_oom_killed(id: &str) -> bool {
    let candidates = [
        format!("/sys/fs/cgroup/runc/{}/memory.events", id),
        format!("/sys/fs/cgroup/{}/memory.events", id),
        format!("/sys/fs/cgroup/system.slice/runc-{}.scope/memory.events", id),
        format!("/sys/fs/cgroup/unified/runc/{}/memory.events", id),
    ];
    for path in &candidates {
        if let Ok(content) = fs::read_to_string(path) {
            for line in content.lines() {
                if line.starts_with("oom_kill ") {
                    if let Some(val_str) = line.split_whitespace().nth(1) {
                        if let Ok(val) = val_str.parse::<i32>() {
                            if val > 0 {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

pub fn container_list_internal(data_dir: &str, auto_restart: bool) -> Result<Vec<ContainerState>, String> {
    let containers_dir = Path::new(data_dir).join("containers");
    if !containers_dir.exists() {
        return Ok(Vec::new());
    }

    let mut list = Vec::new();
    let entries = fs::read_dir(containers_dir).map_err(|e| e.to_string())?;
    for entry in entries {
        if let Ok(entry) = entry {
            if entry.path().is_dir() {
                let id = entry.file_name().to_string_lossy().to_string();
                if let Ok(mut state) = load_container_state(&id) {
                    let output = runc_exec(&["state", &id]);
                    if let Ok(out) = output {
                        if out.status.success() {
                            let out_str = String::from_utf8_lossy(&out.stdout);
                            if let Ok(runc_st) = serde_json::from_str::<serde_json::Value>(&out_str) {
                                let mut runc_status = runc_st.get("status").and_then(|s| s.as_str()).unwrap_or("stopped").to_string();
                                let runc_pid = runc_st.get("pid").and_then(|p| p.as_i64()).unwrap_or(0) as i32;

                                if runc_status == "stopped" && check_oom_killed(&id) {
                                    runc_status = "oom_killed".to_string();
                                }

                                if state.status != runc_status || state.pid != runc_pid {
                                    if state.desired_status.as_deref() == Some("paused") && (runc_status == "stopped" || runc_status == "paused") {
                                        state.status = "paused".to_string();
                                    } else {
                                        state.status = runc_status;
                                        state.pid = runc_pid;
                                    }
                                    let _ = save_container_state(&state);
                                }
                            }
                        } else {
                            if state.status == "running" || state.status == "created" {
                                let is_oom = check_oom_killed(&id);
                                state.status = if is_oom { "oom_killed".to_string() } else { "stopped".to_string() };
                                state.pid = 0;
                                let _ = save_container_state(&state);
                            }
                        }
                    }

                    if auto_restart
                        && (state.status == "stopped" || state.status == "failed")
                        && state.desired_status.as_deref() == Some("running")
                    {
                        if let Some(ref policy) = state.restart_policy {
                            if policy == "always" || policy == "unless-stopped" {
                                if let Err(e) = container_start(&id) {
                                    eprintln!("  ⚠ Auto-restart failed for container {}: {}", id, e);
                                } else if let Ok(new_state) = load_container_state(&id) {
                                    state = new_state;
                                }
                            }
                        }
                    }

                    list.push(state);
                }
            }
        }
    }

    Ok(list)
}

fn generate_config_json(
    bundle_dir: &Path,
    cmd: Vec<String>,
    env: HashMap<String, String>,
    cwd: &str,
    mounts: Vec<String>,
    host_network: bool,
    memory_limit: i64,
    cpu_limit: f64,
    oom_score_adj: Option<i32>,
    read_only: bool,
) -> Result<(), String> {
    let is_rootless = unsafe { libc::getuid() != 0 };

    let mut process_env = vec![
        "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".to_string(),
        "TERM=xterm".to_string(),
        "HOME=/root".to_string(),
    ];
    for (k, v) in env {
        process_env.push(format!("{}={}", k, v));
    }

    let mut oci_mounts = vec![
        json!({
            "destination": "/proc",
            "type": "proc",
            "source": "proc"
        }),
        json!({
            "destination": "/dev",
            "type": "tmpfs",
            "source": "tmpfs",
            "options": ["nosuid", "strictatime", "mode=755", "size=65536k"]
        })
    ];

    if read_only {
        oci_mounts.push(json!({
            "destination": "/tmp",
            "type": "tmpfs",
            "source": "tmpfs",
            "options": ["nosuid", "nodev", "mode=1777", "size=65536k"]
        }));
        oci_mounts.push(json!({
            "destination": "/run",
            "type": "tmpfs",
            "source": "tmpfs",
            "options": ["nosuid", "nodev", "mode=755", "size=65536k"]
        }));
    }

    if !is_rootless {
        oci_mounts.push(json!({
            "destination": "/dev/pts",
            "type": "devpts",
            "source": "devpts",
            "options": ["nosuid", "noexec", "newinstance", "ptmxmode=0666", "mode=0620"]
        }));
        oci_mounts.push(json!({
            "destination": "/dev/shm",
            "type": "tmpfs",
            "source": "shm",
            "options": ["nosuid", "noexec", "nodev", "mode=1777", "size=65536k"]
        }));
        oci_mounts.push(json!({
            "destination": "/sys",
            "type": "sysfs",
            "source": "sysfs",
            "options": ["nosuid", "noexec", "nodev", "ro"]
        }));
    }

    let data_dir = get_data_dir();
    for m in mounts {
        let parts: Vec<&str> = m.splitn(2, ':').collect();
        let (host_path, container_path, is_named_volume) = if parts.len() == 2 {
            let hp = parts[0];
            let cp = parts[1];
            let nv = !hp.starts_with('/') && !hp.starts_with('.') && !hp.starts_with('~');
            (hp.to_string(), cp.to_string(), nv)
        } else {
            let cp = parts[0];
            let container_id = bundle_dir.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "anon".to_string());
            let hp = format!("anon_{}_{}", container_id, cp.replace('/', "_").replace('.', "_"));
            (hp, cp.to_string(), true)
        };

        let resolved_host_path = if is_named_volume {
            Path::new(&data_dir).join("volumes").join(&host_path)
        } else {
            PathBuf::from(&host_path)
        };

        if !resolved_host_path.exists() {
            let _ = fs::create_dir_all(&resolved_host_path);
        }

        let abs_host = fs::canonicalize(&resolved_host_path)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| resolved_host_path.to_string_lossy().to_string());

        oci_mounts.push(json!({
            "destination": container_path,
            "type": "bind",
            "source": abs_host,
            "options": ["bind", "rprivate", "rw"]
        }));
    }

    let mut namespaces = vec![
        json!({ "type": "pid" }),
        json!({ "type": "ipc" }),
        json!({ "type": "uts" }),
        json!({ "type": "mount" })
    ];
    if is_rootless {
        namespaces.push(json!({ "type": "user" }));
    }
    if !host_network {
        namespaces.push(json!({ "type": "network" }));
    }

    let mut resources = json!({});
    if memory_limit > 0 {
        resources["memory"] = json!({ "limit": memory_limit });
    }
    if cpu_limit > 0.0 {
        let period = 100000u64;
        let quota = (cpu_limit * 100000.0) as i64;
        resources["cpu"] = json!({
            "period": period,
            "quota": quota
        });
    }

    let spec = json!({
        "ociVersion": "1.0.2",
        "process": {
            "terminal": false,
            "user": {
                "uid": 0,
                "gid": 0
            },
            "args": cmd,
            "env": process_env,
            "cwd": if cwd.is_empty() { "/" } else { cwd },
            "oomScoreAdj": oom_score_adj,
            "rlimits": [
                {
                    "type": "RLIMIT_NOFILE",
                    "hard": 65536,
                    "soft": 65536
                }
            ],
            "capabilities": if is_rootless { serde_json::Value::Null } else {
                json!({
                    "bounding": [
                        "CAP_AUDIT_WRITE",
                        "CAP_CHOWN",
                        "CAP_DAC_OVERRIDE",
                        "CAP_FOWNER",
                        "CAP_FSETID",
                        "CAP_KILL",
                        "CAP_MKNOD",
                        "CAP_NET_BIND_SERVICE",
                        "CAP_NET_RAW",
                        "CAP_SETGID",
                        "CAP_SETFCAP",
                        "CAP_SETUID",
                        "CAP_SETPCAP",
                        "CAP_SYS_CHROOT"
                    ],
                    "effective": [
                        "CAP_AUDIT_WRITE",
                        "CAP_CHOWN",
                        "CAP_DAC_OVERRIDE",
                        "CAP_FOWNER",
                        "CAP_FSETID",
                        "CAP_KILL",
                        "CAP_MKNOD",
                        "CAP_NET_BIND_SERVICE",
                        "CAP_NET_RAW",
                        "CAP_SETGID",
                        "CAP_SETFCAP",
                        "CAP_SETUID",
                        "CAP_SETPCAP",
                        "CAP_SYS_CHROOT"
                    ],
                    "inheritable": [
                        "CAP_AUDIT_WRITE",
                        "CAP_CHOWN",
                        "CAP_DAC_OVERRIDE",
                        "CAP_FOWNER",
                        "CAP_FSETID",
                        "CAP_KILL",
                        "CAP_MKNOD",
                        "CAP_NET_BIND_SERVICE",
                        "CAP_NET_RAW",
                        "CAP_SETGID",
                        "CAP_SETFCAP",
                        "CAP_SETUID",
                        "CAP_SETPCAP",
                        "CAP_SYS_CHROOT"
                    ],
                    "permitted": [
                        "CAP_AUDIT_WRITE",
                        "CAP_CHOWN",
                        "CAP_DAC_OVERRIDE",
                        "CAP_FOWNER",
                        "CAP_FSETID",
                        "CAP_KILL",
                        "CAP_MKNOD",
                        "CAP_NET_BIND_SERVICE",
                        "CAP_NET_RAW",
                        "CAP_SETGID",
                        "CAP_SETFCAP",
                        "CAP_SETUID",
                        "CAP_SETPCAP",
                        "CAP_SYS_CHROOT"
                    ]
                })
            }
        },
        "root": {
            "path": "rootfs",
            "readonly": read_only
        },
        "hostname": "zenobox",
        "mounts": oci_mounts,
        "linux": {
            "resources": resources,
            "namespaces": namespaces,
            "uidMappings": if is_rootless {
                Some(json!([{ "containerID": 0, "hostID": unsafe { libc::getuid() }, "size": 1 }]))
            } else { None },
            "gidMappings": if is_rootless {
                Some(json!([{ "containerID": 0, "hostID": unsafe { libc::getgid() }, "size": 1 }]))
            } else { None },
            "maskedPaths": if is_rootless { serde_json::Value::Null } else {
                json!([
                    "/proc/acpi", "/proc/asound", "/proc/kcore", "/proc/keys",
                    "/proc/latency_stats", "/proc/timer_list", "/proc/timer_stats",
                    "/proc/sched_debug", "/sys/firmware"
                ])
            },
            "readonlyPaths": if is_rootless { serde_json::Value::Null } else {
                json!([
                    "/proc/bus", "/proc/fs", "/proc/irq", "/proc/sys", "/proc/sysrq-trigger"
                ])
            }
        }
    });

    let config_path = bundle_dir.join("config.json");
    let file = File::create(config_path).map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(file, &spec).map_err(|e| e.to_string())?;

    Ok(())
}

fn is_dir_empty(path: &Path) -> bool {
    if let Ok(mut entries) = fs::read_dir(path) {
        entries.next().is_none()
    } else {
        true
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

pub fn container_create(
    id: &str,
    image: &str,
    cmd: Vec<String>,
    env: HashMap<String, String>,
    cwd: &str,
    mounts: Vec<String>,
    ports: Vec<String>,
    host_network: bool,
    restart_policy: &str,
    memory_limit: i64,
    cpu_limit: f64,
    oom_score_adj: Option<i32>,
    read_only: bool,
    network: &str,
) -> Result<(), String> {
    let data_dir = get_data_dir();
    let state_p = state_file(&data_dir, id);
    if state_p.exists() {
        return Err(format!("Container {} already exists", id));
    }

    let bundle_p = bundle_dir(&data_dir, id);
    fs::create_dir_all(&bundle_p).map_err(|e| e.to_string())?;

    mount_overlayfs(image, &data_dir, id)?;

    let rootfs_p = bundle_p.join("rootfs");
    for m in &mounts {
        let parts: Vec<&str> = m.splitn(2, ':').collect();
        let (host_path, container_path, is_named_volume) = if parts.len() == 2 {
            let hp = parts[0];
            let cp = parts[1];
            let nv = !hp.starts_with('/') && !hp.starts_with('.') && !hp.starts_with('~');
            (hp.to_string(), cp.to_string(), nv)
        } else {
            let cp = parts[0];
            let hp = format!("anon_{}_{}", id, cp.replace('/', "_").replace('.', "_"));
            (hp, cp.to_string(), true)
        };

        if is_named_volume {
            let resolved_host_path = Path::new(&data_dir).join("volumes").join(&host_path);
            let container_src_path = rootfs_p.join(container_path.trim_start_matches('/'));
            
            if !resolved_host_path.exists() {
                let _ = fs::create_dir_all(&resolved_host_path);
            }

            if is_dir_empty(&resolved_host_path) && container_src_path.exists() && container_src_path.is_dir() {
                let _ = copy_dir_all(&container_src_path, &resolved_host_path);
            }
        }
    }

    let mut resolved_cwd = cwd.to_string();
    let mut merged_env = env.clone();

    let img_ref = parse_image_ref(image);
    let cache_dir_name = format!("{}_{}", img_ref.repository, img_ref.tag)
        .replace('/', "_")
        .replace(':', "_");
    let image_config_path = Path::new(&data_dir)
        .join("images")
        .join(&cache_dir_name)
        .join("image-config.json");
    if image_config_path.exists() {
        if let Ok(file) = File::open(&image_config_path) {
            if let Ok(cfg) = serde_json::from_reader::<_, serde_json::Value>(file) {
                if resolved_cwd.is_empty() {
                    if let Some(workdir) = cfg.get("config")
                        .and_then(|c| c.get("WorkingDir"))
                        .and_then(|w| w.as_str()) 
                    {
                        resolved_cwd = workdir.to_string();
                    }
                }

                if let Some(env_array) = cfg.get("config")
                    .and_then(|c| c.get("Env"))
                    .and_then(|e| e.as_array())
                {
                    for item in env_array {
                        if let Some(env_str) = item.as_str() {
                            let parts: Vec<&str> = env_str.splitn(2, '=').collect();
                            if parts.len() == 2 {
                                let k = parts[0].to_string();
                                let v = parts[1].to_string();
                                merged_env.entry(k).or_insert(v);
                            }
                        }
                    }
                }
            }
        }
    }

    generate_config_json(
        &bundle_p,
        cmd.clone(),
        merged_env.clone(),
        &resolved_cwd,
        mounts.clone(),
        host_network,
        memory_limit,
        cpu_limit,
        oom_score_adj,
        read_only,
    )?;

    let c_log_path = log_path(&data_dir, id).to_string_lossy().to_string();
    let state = ContainerState {
        id: id.to_string(),
        image: image.to_string(),
        status: "created".to_string(),
        pid: 0,
        created_at: chrono::Utc::now().to_rfc3339(),
        exited_at: None,
        exit_code: None,
        cmd,
        log_path: Some(c_log_path),
        ports: Some(ports),
        env: Some(merged_env),
        mounts: Some(mounts),
        cwd: Some(resolved_cwd),
        host_network: Some(host_network),
        restart_policy: Some(restart_policy.to_string()),
        desired_status: Some("stopped".to_string()),
        memory_limit: Some(memory_limit),
        cpu_limit: Some(cpu_limit),
        oom_score_adj,
        read_only: Some(read_only),
        network: Some(network.to_string()),
    };

    save_container_state(&state)?;

    Ok(())
}

pub fn container_start(id: &str) -> Result<(), String> {
    let data_dir = get_data_dir();
    let mut state = load_container_state(id)?;

    if let Ok(out) = runc_exec(&["state", id]) {
        if out.status.success() {
            let out_str = String::from_utf8_lossy(&out.stdout);
            if let Ok(runc_st) = serde_json::from_str::<serde_json::Value>(&out_str) {
                if let Some(r_status) = runc_st.get("status").and_then(|s| s.as_str()) {
                    if r_status == "running" {
                        state.status = "running".to_string();
                        let _ = save_container_state(&state);
                        return Err(format!("Container {} is already running", id));
                    }
                }
            }
        }
    }

    mount_overlayfs(&state.image, &data_dir, id)?;

    let old_ip = state.env.as_ref().and_then(|e| e.get("ZENO_IP").cloned()).unwrap_or_default();
    let old_ports = state.ports.clone().unwrap_or_default();
    clean_container_network(id, &old_ip, &old_ports);

    let bundle_p = bundle_dir(&data_dir, id);

    let _ = runc_exec(&["delete", "--force", id]);

    let log_p = log_path(&data_dir, id);
    rotate_log_file_if_needed(&log_p, 10 * 1024 * 1024, 3);
    let log_file = File::options().create(true).append(true).open(&log_p).map_err(|e| format!("Failed to create log file: {}", e))?;

    let runc_bin = get_runc_bin();
    let root = format!("{}/runc", get_data_dir());
    let run_create_status = Command::new(&runc_bin)
        .args(&["--root", &root, "create", "-b", &bundle_p.to_string_lossy(), id])
        .stdout(log_file.try_clone().map_err(|e| e.to_string())?)
        .stderr(log_file)
        .status()
        .map_err(|e| format!("runc create process failed: {}", e))?;

    if !run_create_status.success() {
        state.status = "failed".to_string();
        let _ = save_container_state(&state);
        let err_msg = fs::read_to_string(&log_p).unwrap_or_default();
        return Err(format!("runc create failed: {}", err_msg));
    }

    let mut runc_pid = 0;
    if let Ok(out) = runc_exec(&["state", id]) {
        if out.status.success() {
            let out_str = String::from_utf8_lossy(&out.stdout);
            if let Ok(runc_st) = serde_json::from_str::<serde_json::Value>(&out_str) {
                runc_pid = runc_st.get("pid").and_then(|p| p.as_i64()).unwrap_or(0) as i32;
            }
        }
    }

    if runc_pid > 0 {
        state.pid = runc_pid;
        let is_host_net = state.host_network.unwrap_or(false);
        if !is_host_net {
            let ports = state.ports.clone().unwrap_or_default();
            let net_name = state.network.clone().unwrap_or_default();
            match configure_container_network(&data_dir, id, runc_pid, ports, &net_name) {
                Ok(ip) => {
                    let mut env = state.env.clone().unwrap_or_default();
                    env.insert("ZENO_IP".to_string(), ip);
                    state.env = Some(env);
                }
                Err(e) => {
                    eprintln!("  ⚠ Network configuration failed: {}", e);
                }
            }
        }
    }

    let run_start = runc_exec(&["start", id])
        .map_err(|e| format!("runc start process failed: {}", e))?;
    if !run_start.status.success() {
        let ip = state.env.as_ref().and_then(|e| e.get("ZENO_IP").cloned()).unwrap_or_default();
        let ports = state.ports.clone().unwrap_or_default();
        clean_container_network(id, &ip, &ports);

        state.status = "failed".to_string();
        let _ = save_container_state(&state);
        let err_msg = String::from_utf8_lossy(&run_start.stderr);
        return Err(format!("runc start failed: {}", err_msg));
    }

    state.status = "running".to_string();
    state.desired_status = Some("running".to_string());
    state.exit_code = Some(0);
    save_container_state(&state)?;

    let _ = sync_hosts_entries(&data_dir);
    Ok(())
}

pub fn container_pause(id: &str) -> Result<(), String> {
    let mut state = load_container_state(id)?;
    let out = runc_exec(&["pause", &state.id]).map_err(|e| format!("runc pause failed: {}", e))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        eprintln!("[container_pause] runc pause stderr: {}", err);
        return Err(format!("runc pause failed: {}", err));
    }
    state.status = "paused".to_string();
    state.desired_status = Some("paused".to_string());
    save_container_state(&state)?;
    Ok(())
}

pub fn container_unpause(id: &str) -> Result<(), String> {
    let mut state = load_container_state(id)?;
    let res = runc_exec(&["resume", &state.id]);
    let success = res.as_ref().map(|o| o.status.success()).unwrap_or(false);
    if !success {
        let _ = container_start(&state.id);
    } else {
        state.status = "running".to_string();
        state.desired_status = Some("running".to_string());
        save_container_state(&state)?;
    }
    Ok(())
}

pub fn container_stop(id: &str) -> Result<(), String> {
    let data_dir = get_data_dir();
    let mut state = load_container_state(id)?;
    if state.status != "running" && state.status != "paused" {
        return Ok(());
    }

    let kill_term = runc_exec(&["kill", &state.id, "SIGTERM"]);
    if kill_term.is_err() || !kill_term.unwrap().status.success() {
        let _ = runc_exec(&["kill", &state.id, "SIGKILL"]);
    }

    let ip = state.env.as_ref().and_then(|e| e.get("ZENO_IP").cloned()).unwrap_or_default();
    let ports = state.ports.clone().unwrap_or_default();
    clean_container_network(&state.id, &ip, &ports);

    state.status = "stopped".to_string();
    state.desired_status = Some("stopped".to_string());
    state.pid = 0;
    save_container_state(&state)?;

    let _ = sync_hosts_entries(&data_dir);
    Ok(())
}

pub fn container_delete(id: &str) -> Result<(), String> {
    let data_dir = get_data_dir();
    let state = load_container_state(id);
    if let Ok(state) = state {
        let ip = state.env.as_ref().and_then(|e| e.get("ZENO_IP").cloned()).unwrap_or_default();
        let ports = state.ports.clone().unwrap_or_default();
        clean_container_network(id, &ip, &ports);
    }

    let _ = runc_exec(&["kill", id, "SIGKILL"]);
    let _ = runc_exec(&["delete", "--force", id]);

    let dst_rootfs = rootfs_dir(&data_dir, id);
    if dst_rootfs.exists() {
        let _ = run_privileged_status("umount", &["-l", &dst_rootfs.to_string_lossy().to_string()]);
        std::thread::sleep(std::time::Duration::from_millis(150));
    }

    let cont_p = container_dir(&data_dir, id);
    
    let mut delete_err = None;
    for attempt in 1..=5 {
        match fs::remove_dir_all(&cont_p) {
            Ok(_) => {
                delete_err = None;
                break;
            }
            Err(e) => {
                delete_err = Some(e);
                std::thread::sleep(std::time::Duration::from_millis(100 * attempt));
            }
        }
    }

    if let Some(err) = delete_err {
        return Err(format!("Failed to delete container directory '{}': {}", cont_p.display(), err));
    }

    let _ = sync_hosts_entries(&data_dir);
    Ok(())
}

pub fn container_update(id: &str, memory_limit: i64, cpu_limit: f64) -> Result<(), String> {
    let data_dir = get_data_dir();
    let mut state = load_container_state(id)?;

    let mut runc_args = vec!["update"];
    let mem_str = memory_limit.to_string();
    if memory_limit > 0 {
        runc_args.push("--memory");
        runc_args.push(&mem_str);
    }
    let period_str = "100000".to_string();
    let quota = (cpu_limit * 100000.0) as i64;
    let quota_str = quota.to_string();
    if cpu_limit > 0.0 {
        runc_args.push("--cpu-period");
        runc_args.push(&period_str);
        runc_args.push("--cpu-quota");
        runc_args.push(&quota_str);
    }
    runc_args.push(id);

    if state.status == "running" {
        let run_upd = runc_exec(&runc_args)
            .map_err(|e| format!("runc update failed: {}", e))?;
        if !run_upd.status.success() {
            let err_msg = String::from_utf8_lossy(&run_upd.stderr);
            return Err(format!("runc update failed: {}", err_msg));
        }
    }

    let config_path = bundle_dir(&data_dir, id).join("config.json");
    if config_path.exists() {
        if let Ok(data) = fs::read_to_string(&config_path) {
            if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(&data) {
                if memory_limit > 0 {
                    val["linux"]["resources"]["memory"] = json!({ "limit": memory_limit });
                }
                if cpu_limit > 0.0 {
                    val["linux"]["resources"]["cpu"] = json!({
                        "period": 100000u64,
                        "quota": quota
                    });
                }
                if let Ok(new_data) = serde_json::to_string_pretty(&val) {
                    let _ = fs::write(&config_path, new_data);
                }
            }
        }
    }

    if memory_limit > 0 { state.memory_limit = Some(memory_limit); }
    if cpu_limit > 0.0 { state.cpu_limit = Some(cpu_limit); }
    save_container_state(&state)?;

    Ok(())
}

pub fn container_logs(id: &str) -> Result<String, String> {
    let data_dir = get_data_dir();
    let l_path = log_path(&data_dir, id);
    if l_path.exists() {
        fs::read_to_string(l_path).map_err(|e| e.to_string())
    } else {
        Err(format!("Log file for container {} not found", id))
    }
}

pub fn container_exec(id: &str, cmd: &[&str]) -> Result<String, String> {
    let mut runc_args = vec!["exec", id];
    runc_args.extend_from_slice(cmd);
    let output = runc_exec(&runc_args).map_err(|e| format!("Exec process failed: {}", e))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        Err(if err.is_empty() { "Exec returned non-zero exit code".to_string() } else { err })
    }
}

pub fn read_container_stats(id: &str) -> Result<serde_json::Value, String> {
    let state = load_container_state(id)?;
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);

    let mut mem_usage: u64 = 0;
    let mut mem_limit: u64 = state.memory_limit.unwrap_or(0) as u64;
    if mem_limit == 0 {
        mem_limit = 1024 * 1024 * 1024 * 8;
    }

    let mem_paths = [
        format!("/sys/fs/cgroup/runc/{}/memory.current", id),
        format!("/sys/fs/cgroup/{}/memory.current", id),
        format!("/sys/fs/cgroup/memory/runc/{}/memory.usage_in_bytes", id),
        format!("/sys/fs/cgroup/memory/{}/memory.usage_in_bytes", id),
    ];
    for p in &mem_paths {
        if let Ok(val_str) = fs::read_to_string(p) {
            if let Ok(v) = val_str.trim().parse::<u64>() {
                mem_usage = v;
                break;
            }
        }
    }

    let limit_paths = [
        format!("/sys/fs/cgroup/runc/{}/memory.max", id),
        format!("/sys/fs/cgroup/{}/memory.max", id),
        format!("/sys/fs/cgroup/memory/runc/{}/memory.limit_in_bytes", id),
    ];
    for p in &limit_paths {
        if let Ok(val_str) = fs::read_to_string(p) {
            let trimmed = val_str.trim();
            if trimmed != "max" {
                if let Ok(v) = trimmed.parse::<u64>() {
                    if v < 1 << 60 {
                        mem_limit = v;
                        break;
                    }
                }
            }
        }
    }

    let mut total_usage_ns: u64 = 0;
    let cpu_paths = [
        format!("/sys/fs/cgroup/runc/{}/cpu.stat", id),
        format!("/sys/fs/cgroup/{}/cpu.stat", id),
    ];
    for p in &cpu_paths {
        if let Ok(content) = fs::read_to_string(p) {
            for line in content.lines() {
                if line.starts_with("usage_usec ") {
                    if let Some(val_str) = line.split_whitespace().nth(1) {
                        if let Ok(usec) = val_str.parse::<u64>() {
                            total_usage_ns = usec * 1000;
                        }
                    }
                }
            }
            if total_usage_ns > 0 { break; }
        }
    }
    if total_usage_ns == 0 {
        let cpuacct_paths = [
            format!("/sys/fs/cgroup/cpu/runc/{}/cpuacct.usage", id),
            format!("/sys/fs/cgroup/cpu/{}/cpuacct.usage", id),
        ];
        for p in &cpuacct_paths {
            if let Ok(val_str) = fs::read_to_string(p) {
                if let Ok(v) = val_str.trim().parse::<u64>() {
                    total_usage_ns = v;
                    break;
                }
            }
        }
    }

    let pids = if state.pid > 0 { 1 } else { 0 };

    Ok(json!({
        "read": now,
        "preread": now,
        "pids_stats": {
            "current": pids
        },
        "blkio_stats": {
            "io_service_bytes_recursive": []
        },
        "num_procs": pids,
        "storage_stats": {},
        "cpu_stats": {
            "cpu_usage": {
                "total_usage": total_usage_ns,
                "percpu_usage": [total_usage_ns],
                "usage_in_kernelmode": 0,
                "usage_in_usermode": 0
            },
            "system_cpu_usage": 1000000000000u64,
            "online_cpus": 1
        },
        "precpu_stats": {
            "cpu_usage": {
                "total_usage": total_usage_ns.saturating_sub(100000),
                "percpu_usage": [total_usage_ns.saturating_sub(100000)],
                "usage_in_kernelmode": 0,
                "usage_in_usermode": 0
            },
            "system_cpu_usage": 999900000000u64,
            "online_cpus": 1
        },
        "memory_stats": {
            "usage": mem_usage,
            "max_usage": mem_usage,
            "limit": mem_limit,
            "stats": {}
        },
        "name": format!("/{}", state.id),
        "id": id,
        "networks": {}
    }))
}

