use std::collections::HashMap;
use std::path::Path;
use std::fs::{self, File};
use serde::{Serialize, Deserialize};

use crate::utils::{get_data_dir, container_dir, rootfs_dir, parse_image_ref};
use crate::container::{
    container_create, container_start, container_stop, container_delete
};
use crate::image::{pull_image, get_image_default_cmd};

#[derive(Serialize, Debug, Clone)]
pub struct ComposeExtraHosts(pub Vec<String>);

impl<'de> serde::Deserialize<'de> for ComposeExtraHosts {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ExtraHostsVisitor;
        impl<'de> serde::de::Visitor<'de> for ExtraHostsVisitor {
            type Value = ComposeExtraHosts;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a sequence of strings or a map of hostnames to IPs")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut hosts = Vec::new();
                while let Some(elem) = seq.next_element::<String>()? {
                    hosts.push(elem);
                }
                Ok(ComposeExtraHosts(hosts))
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: serde::de::MapAccess<'de>,
            {
                let mut hosts = Vec::new();
                while let Some((k, v)) = map.next_entry::<String, String>()? {
                    hosts.push(format!("{}:{}", k, v));
                }
                Ok(ComposeExtraHosts(hosts))
            }
        }
        deserializer.deserialize_any(ExtraHostsVisitor)
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct ComposeCommand(pub Vec<String>);

impl<'de> serde::Deserialize<'de> for ComposeCommand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct CmdVisitor;
        impl<'de> serde::de::Visitor<'de> for CmdVisitor {
            type Value = ComposeCommand;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string or a sequence of strings")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(ComposeCommand(v.split_whitespace().map(|s| s.to_string()).collect()))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut cmd = Vec::new();
                while let Some(elem) = seq.next_element::<String>()? {
                    cmd.push(elem);
                }
                Ok(ComposeCommand(cmd))
            }
        }
        deserializer.deserialize_any(CmdVisitor)
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct ComposeEnvironment(pub HashMap<String, String>);

impl<'de> serde::Deserialize<'de> for ComposeEnvironment {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct EnvVisitor;
        impl<'de> serde::de::Visitor<'de> for EnvVisitor {
            type Value = ComposeEnvironment;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a map or a sequence of strings")
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: serde::de::MapAccess<'de>,
            {
                let mut env = HashMap::new();
                while let Some((k, v)) = map.next_entry::<String, serde_json::Value>()? {
                    let v_str = match v {
                        serde_json::Value::String(s) => s,
                        serde_json::Value::Number(n) => n.to_string(),
                        serde_json::Value::Bool(b) => b.to_string(),
                        serde_json::Value::Null => String::new(),
                        _ => v.to_string(),
                    };
                    env.insert(k, v_str);
                }
                Ok(ComposeEnvironment(env))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut env = HashMap::new();
                while let Some(item) = seq.next_element::<serde_json::Value>()? {
                    let item_str = match item {
                        serde_json::Value::String(s) => s,
                        serde_json::Value::Number(n) => n.to_string(),
                        serde_json::Value::Bool(b) => b.to_string(),
                        _ => continue,
                    };
                    let parts: Vec<&str> = item_str.splitn(2, '=').collect();
                    if parts.len() == 2 {
                        env.insert(parts[0].to_string(), parts[1].to_string());
                    } else if parts.len() == 1 {
                        env.insert(parts[0].to_string(), String::new());
                    }
                }
                Ok(ComposeEnvironment(env))
            }
        }
        deserializer.deserialize_any(EnvVisitor)
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct ComposePorts(pub Vec<String>);

impl<'de> serde::Deserialize<'de> for ComposePorts {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct PortsVisitor;
        impl<'de> serde::de::Visitor<'de> for PortsVisitor {
            type Value = ComposePorts;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a sequence of strings, integers, or objects mapping ports")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut ports = Vec::new();
                while let Some(elem) = seq.next_element::<serde_json::Value>()? {
                    match elem {
                        serde_json::Value::String(s) => ports.push(s),
                        serde_json::Value::Number(n) => ports.push(n.to_string()),
                        serde_json::Value::Object(obj) => {
                            let target = obj.get("target")
                                .map(|v| match v {
                                    serde_json::Value::Number(n) => n.to_string(),
                                    serde_json::Value::String(s) => s.clone(),
                                    _ => String::new(),
                                })
                                .unwrap_or_default();
                            
                            let published = obj.get("published")
                                .map(|v| match v {
                                    serde_json::Value::Number(n) => n.to_string(),
                                    serde_json::Value::String(s) => s.clone(),
                                    _ => String::new(),
                                });

                            let host_ip = obj.get("host_ip")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());

                            let protocol = obj.get("protocol")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());

                            if !target.is_empty() {
                                let mut port_str = String::new();
                                if let Some(ip) = host_ip {
                                    port_str.push_str(&format!("{}:", ip));
                                }
                                if let Some(pub_port) = published {
                                    port_str.push_str(&format!("{}:", pub_port));
                                }
                                port_str.push_str(&target);
                                if let Some(proto) = protocol {
                                    port_str.push_str(&format!("/{}", proto));
                                }
                                ports.push(port_str);
                            }
                        }
                        _ => {}
                    }
                }
                Ok(ComposePorts(ports))
            }
        }
        deserializer.deserialize_seq(PortsVisitor)
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct ComposeDependsOn(pub Vec<String>);

impl<'de> serde::Deserialize<'de> for ComposeDependsOn {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct DependsOnVisitor;
        impl<'de> serde::de::Visitor<'de> for DependsOnVisitor {
            type Value = ComposeDependsOn;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a sequence of service names or a map of service dependencies")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut deps = Vec::new();
                while let Some(elem) = seq.next_element::<String>()? {
                    deps.push(elem);
                }
                Ok(ComposeDependsOn(deps))
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: serde::de::MapAccess<'de>,
            {
                let mut deps = Vec::new();
                while let Some((k, _v)) = map.next_entry::<String, serde_yaml::Value>()? {
                    deps.push(k);
                }
                Ok(ComposeDependsOn(deps))
            }
        }
        deserializer.deserialize_any(DependsOnVisitor)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ComposeLimits {
    pub memory: Option<String>,
    pub cpus: Option<serde_yaml::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ComposeResources {
    pub limits: Option<ComposeLimits>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ComposeDeploy {
    pub resources: Option<ComposeResources>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ComposeHealthCheck {
    pub test: serde_yaml::Value,
    pub interval: Option<String>,
    pub timeout: Option<String>,
    pub retries: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ComposeService {
    pub image: Option<String>,
    pub container_name: Option<String>,
    pub ports: Option<ComposePorts>,
    pub environment: Option<ComposeEnvironment>,
    pub env_file: Option<serde_yaml::Value>,
    pub volumes: Option<Vec<String>>,
    pub entrypoint: Option<ComposeCommand>,
    pub command: Option<ComposeCommand>,
    pub depends_on: Option<ComposeDependsOn>,
    pub networks: Option<Vec<String>>,
    pub restart: Option<String>,
    pub healthcheck: Option<ComposeHealthCheck>,
    pub mem_limit: Option<String>,
    pub cpus: Option<f64>,
    pub deploy: Option<ComposeDeploy>,
    pub oom_score_adj: Option<i32>,
    pub read_only: Option<bool>,
    pub network_mode: Option<String>,
    pub extra_hosts: Option<ComposeExtraHosts>,
    pub working_dir: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ComposeNetwork {
    pub driver: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ComposeFile {
    pub version: Option<String>,
    pub services: HashMap<String, ComposeService>,
    pub networks: Option<HashMap<String, ComposeNetwork>>,
    pub volumes: Option<HashMap<String, serde_yaml::Value>>,
}

fn order_services(services: &HashMap<String, ComposeService>) -> Vec<String> {
    let mut ordered = Vec::new();
    let mut visited = std::collections::HashSet::new();

    fn visit(
        name: &str,
        services: &HashMap<String, ComposeService>,
        visited: &mut std::collections::HashSet<String>,
        ordered: &mut Vec<String>,
    ) {
        if visited.contains(name) {
            return;
        }
        visited.insert(name.to_string());
        if let Some(svc) = services.get(name) {
            if let Some(ref deps) = svc.depends_on {
                for dep in &deps.0 {
                    if services.contains_key(dep) {
                        visit(dep, services, visited, ordered);
                    }
                }
            }
        }
        ordered.push(name.to_string());
    }

    let mut keys: Vec<String> = services.keys().cloned().collect();
    keys.sort();

    for k in keys {
        visit(&k, services, &mut visited, &mut ordered);
    }

    ordered
}

fn parse_memory_bytes(m_str: &str) -> i64 {
    if m_str.is_empty() {
        return 0;
    }
    let clean = m_str.trim().to_lowercase();
    let mut unit: i64 = 1;
    let mut num_str = clean.as_str();
    if num_str.ends_with('b') {
        num_str = &num_str[..num_str.len() - 1];
    }
    if num_str.ends_with('k') {
        unit = 1024;
        num_str = &num_str[..num_str.len() - 1];
    } else if num_str.ends_with('m') {
        unit = 1024 * 1024;
        num_str = &num_str[..num_str.len() - 1];
    } else if num_str.ends_with('g') {
        unit = 1024 * 1024 * 1024;
        num_str = &num_str[..num_str.len() - 1];
    }

    num_str.parse::<i64>().unwrap_or(0) * unit
}

fn load_env_file(compose_path: &str, env_file_val: Option<&serde_yaml::Value>) -> HashMap<String, String> {
    let mut env = HashMap::new();
    let compose_path_buf = Path::new(compose_path);
    let parent_dir = compose_path_buf.parent().unwrap_or_else(|| Path::new("."));

    let dot_env = parent_dir.join(".env");
    if dot_env.exists() {
        if let Ok(content) = fs::read_to_string(&dot_env) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                let parts: Vec<&str> = trimmed.splitn(2, '=').collect();
                if parts.len() == 2 {
                    let k = parts[0].trim().to_string();
                    let v = parts[1].trim().to_string();
                    let v_clean = if (v.starts_with('"') && v.ends_with('"')) || (v.starts_with('\'') && v.ends_with('\'')) {
                        if v.len() >= 2 { v[1..v.len()-1].to_string() } else { v }
                    } else {
                        v
                    };
                    env.insert(k, v_clean);
                }
            }
        }
    }

    if let Some(val) = env_file_val {
        let files = match val {
            serde_yaml::Value::String(s) => vec![s.clone()],
            serde_yaml::Value::Sequence(seq) => {
                let mut v = Vec::new();
                for item in seq {
                    if let Some(s) = item.as_str() {
                        v.push(s.to_string());
                    }
                }
                v
            }
            _ => Vec::new(),
        };

        for file_name in files {
            let f_path = parent_dir.join(&file_name);
            if let Ok(content) = fs::read_to_string(f_path) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }
                    let parts: Vec<&str> = trimmed.splitn(2, '=').collect();
                    if parts.len() == 2 {
                        let k = parts[0].trim().to_string();
                        let v = parts[1].trim().to_string();
                        let v_clean = if (v.starts_with('"') && v.ends_with('"')) || (v.starts_with('\'') && v.ends_with('\'')) {
                            if v.len() >= 2 {
                                v[1..v.len()-1].to_string()
                            } else {
                                v
                            }
                        } else {
                            v
                        };
                        env.insert(k, v_clean);
                    }
                }
            }
        }
    }

    env
}

fn inject_hosts_entries(
    data_dir: &str,
    container_id: &str,
    services: &HashMap<String, ComposeService>,
    current_name: &str,
) -> Result<(), String> {
    let hosts_path = rootfs_dir(data_dir, container_id).join("etc/hosts");
    let mut data = fs::read_to_string(&hosts_path).unwrap_or_else(|_| "127.0.0.1 localhost\n".to_string());

    let mut entries = Vec::new();
    for (svc_name, svc) in services {
        if svc_name == current_name {
            continue;
        }
        let cn = svc.container_name.as_ref().unwrap_or(svc_name);
        entries.push(format!("127.0.0.1\t{}\t{}", cn, svc_name));
    }

    if let Some(svc) = services.get(current_name) {
        if let Some(ref eh) = svc.extra_hosts {
            for entry in &eh.0 {
                let parts: Vec<&str> = entry.splitn(2, ':').collect();
                if parts.len() == 2 {
                    let hostname = parts[0].trim();
                    let ip = parts[1].trim();
                    entries.push(format!("{}\t{}", ip, hostname));
                }
            }
        }
    }

    if entries.is_empty() {
        return Ok(());
    }

    data.push_str("\n# Zenobox compose service discovery\n");
    for e in entries {
        data.push_str(&format!("{}\n", e));
    }

    fs::write(hosts_path, data).map_err(|e| e.to_string())?;
    Ok(())
}

fn expand_env_vars(s: &str, loaded_env: &HashMap<String, String>) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '$' {
            if chars.peek() == Some(&'{') {
                chars.next();
                let mut var_expr = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch == '}' {
                        chars.next();
                        break;
                    }
                    var_expr.push(chars.next().unwrap());
                }

                if let Some(idx) = var_expr.find(":-") {
                    let var_name = &var_expr[..idx];
                    let default_val = &var_expr[idx + 2..];
                    let val = loaded_env.get(var_name)
                        .cloned()
                        .or_else(|| std::env::var(var_name).ok())
                        .filter(|v| !v.is_empty())
                        .unwrap_or_else(|| default_val.to_string());
                    result.push_str(&val);
                } else {
                    let val = loaded_env.get(&var_expr)
                        .cloned()
                        .or_else(|| std::env::var(&var_expr).ok())
                        .unwrap_or_default();
                    result.push_str(&val);
                }
            } else {
                let mut var_name = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch.is_alphanumeric() || ch == '_' {
                        var_name.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                if var_name.is_empty() {
                    result.push('$');
                } else {
                    let val = loaded_env.get(&var_name)
                        .cloned()
                        .or_else(|| std::env::var(&var_name).ok())
                        .unwrap_or_default();
                    result.push_str(&val);
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

pub fn compose_up(path: &str) -> Result<String, String> {
    let data_dir = get_data_dir();
    let f = File::open(path).map_err(|e| format!("Failed to read compose file: {}", e))?;
    let cf: ComposeFile = serde_yaml::from_reader(f).map_err(|e| format!("Failed to parse YAML: {}", e))?;

    let compose_path_buf = Path::new(path);
    let project_name = compose_path_buf
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "default".to_string());

    let ordered = order_services(&cf.services);
    let mut output = String::new();

    for name in ordered {
        let svc = &cf.services[&name];

        let loaded_env = load_env_file(path, svc.env_file.as_ref());

        let raw_image = svc.image.as_ref().ok_or_else(|| format!("Service {} has no image", name))?;
        let image = expand_env_vars(raw_image, &loaded_env);
        output.push_str(&format!("▶ Service: {} (image: {:?})\n", name, image));

        let img_ref = parse_image_ref(&image);
        let cache_dir_name = format!("{}_{}", img_ref.repository, img_ref.tag)
            .replace('/', "_")
            .replace(':', "_");
        let cache_dir = Path::new(&data_dir).join("images").join(&cache_dir_name);
        let default_cmd = if !cache_dir.exists() {
            output.push_str(&format!("  ▶ Image {} not found locally. Pulling...\n", image));
            let rt = tokio::runtime::Handle::current();
            let pull_res = tokio::task::block_in_place(|| {
                rt.block_on(async { pull_image(&image).await })
            });
            match pull_res {
                Ok(cmd) => cmd,
                Err(e) => {
                    return Err(format!("Failed to pull image {}: {}", image, e));
                }
            }
        } else {
            get_image_default_cmd(&image)
        };

        let raw_container_name = svc.container_name.as_ref().unwrap_or(&name);
        let container_name_expanded = expand_env_vars(raw_container_name, &loaded_env);
        let container_name = if container_name_expanded.is_empty() { name.clone() } else { container_name_expanded };

        let cont_p = container_dir(&data_dir, &container_name);
        if cont_p.exists() {
            output.push_str(&format!("  ▶ Container '{}' already exists. Stopping and removing first...\n", container_name));
            let _ = container_stop(&container_name);
            let _ = container_delete(&container_name);
        }

        let cmd_args = match (&svc.entrypoint, &svc.command) {
            (Some(entrypoint), Some(command)) => {
                let mut cmd = Vec::new();
                for arg in &entrypoint.0 {
                    cmd.push(expand_env_vars(arg, &loaded_env));
                }
                for arg in &command.0 {
                    cmd.push(expand_env_vars(arg, &loaded_env));
                }
                cmd
            }
            (Some(entrypoint), None) => {
                let mut cmd = Vec::new();
                for arg in &entrypoint.0 {
                    cmd.push(expand_env_vars(arg, &loaded_env));
                }
                cmd
            }
            (None, Some(command)) => {
                let mut cmd = Vec::new();
                if !default_cmd.is_empty() {
                    let first = &default_cmd[0];
                    if first.contains("entrypoint") || first.ends_with(".sh") {
                        cmd.push(first.clone());
                    }
                }
                for arg in &command.0 {
                    cmd.push(expand_env_vars(arg, &loaded_env));
                }
                cmd
            }
            (None, None) => {
                default_cmd
            }
        };

        let mut env = HashMap::new();
        if let Some(ref e) = svc.environment {
            for (k, v) in &e.0 {
                let expanded_v = expand_env_vars(v, &loaded_env);
                env.insert(k.clone(), expanded_v);
            }
        }
        for (k, v) in &loaded_env {
            env.entry(k.clone()).or_insert(v.clone());
        }

        let mut volumes = Vec::new();
        if let Some(ref vols) = svc.volumes {
            for v_raw in vols {
                let v = expand_env_vars(v_raw, &loaded_env);
                let parts: Vec<&str> = v.splitn(2, ':').collect();
                if parts.len() == 2 {
                    let host_path = parts[0];
                    let container_path = parts[1];
                    let is_named_volume = !host_path.starts_with('/') && !host_path.starts_with('.') && !host_path.starts_with('~');
                    if is_named_volume {
                        let mut final_host_path = host_path.to_string();
                        let mut use_prefix = true;

                        if let Some(ref top_volumes) = cf.volumes {
                            if let Some(vol_config) = top_volumes.get(host_path) {
                                if let Some(map) = vol_config.as_mapping() {
                                    if let Some(ext_val) = map.get(&serde_yaml::Value::String("external".to_string())) {
                                        if ext_val.as_bool() == Some(true) {
                                            use_prefix = false;
                                        }
                                    }
                                    if let Some(name_val) = map.get(&serde_yaml::Value::String("name".to_string())) {
                                        if let Some(custom_name) = name_val.as_str() {
                                            final_host_path = custom_name.to_string();
                                            use_prefix = false;
                                        }
                                    }
                                }
                            }
                        }

                        if use_prefix {
                            volumes.push(format!("{}_{}:{}", project_name, final_host_path, container_path));
                        } else {
                            volumes.push(format!("{}:{}", final_host_path, container_path));
                        }
                    } else {
                        let resolved_path = if host_path.starts_with('.') {
                            if let Some(parent) = compose_path_buf.parent() {
                                let abs_path = parent.join(host_path);
                                if let Ok(canonical) = fs::canonicalize(&abs_path) {
                                    canonical.to_string_lossy().to_string()
                                } else {
                                    abs_path.to_string_lossy().to_string()
                                }
                            } else {
                                host_path.to_string()
                            }
                        } else {
                            host_path.to_string()
                        };
                        volumes.push(format!("{}:{}", resolved_path, container_path));
                    }
                } else {
                    volumes.push(v);
                }
            }
        }

        let mut ports = Vec::new();
        if let Some(ref p) = svc.ports {
            for raw_port in &p.0 {
                let expanded = expand_env_vars(raw_port, &loaded_env);
                if !expanded.is_empty() {
                    ports.push(expanded);
                }
            }
        }

        let restart_policy = svc.restart.as_deref().unwrap_or("no");
        let mem_limit = if let Some(ref limit) = svc.mem_limit {
            parse_memory_bytes(&expand_env_vars(limit, &loaded_env))
        } else if let Some(ref d) = svc.deploy {
            if let Some(ref res) = d.resources {
                if let Some(ref lim) = res.limits {
                    if let Some(ref mem) = lim.memory {
                        parse_memory_bytes(&expand_env_vars(mem, &loaded_env))
                    } else { 0 }
                } else { 0 }
            } else { 0 }
        } else {
            0
        };
        let cpu_limit = if let Some(c) = svc.cpus {
            c
        } else if let Some(ref d) = svc.deploy {
            if let Some(ref res) = d.resources {
                if let Some(ref lim) = res.limits {
                    if let Some(ref c_val) = lim.cpus {
                        match c_val {
                            serde_yaml::Value::Number(n) => n.as_f64().unwrap_or(0.0),
                            serde_yaml::Value::String(s) => {
                                expand_env_vars(s, &loaded_env).parse::<f64>().unwrap_or(0.0)
                            }
                            _ => 0.0,
                        }
                    } else { 0.0 }
                } else { 0.0 }
            } else { 0.0 }
        } else {
            0.0
        };
        let read_only = svc.read_only.unwrap_or(false);
        let network_name = if let Some(ref nets) = svc.networks {
            if !nets.is_empty() {
                &nets[0]
            } else {
                "bridge"
            }
        } else {
            "bridge"
        };

        let abs_config_path = fs::canonicalize(compose_path_buf)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| path.to_string());
        let abs_workdir = compose_path_buf
            .parent()
            .and_then(|p| fs::canonicalize(p).ok())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string());

        let mut compose_labels = HashMap::new();
        compose_labels.insert("com.docker.compose.project".to_string(), project_name.clone());
        compose_labels.insert("com.docker.compose.service".to_string(), name.clone());
        compose_labels.insert("com.docker.compose.project.config_files".to_string(), abs_config_path);
        compose_labels.insert("com.docker.compose.project.working_dir".to_string(), abs_workdir);

        output.push_str(&format!("  ▶ Creating container '{}'...\n", container_name));
        let is_host_net = svc.network_mode.as_deref() == Some("host") || network_name == "host";
        container_create(
            &container_name,
            &image,
            cmd_args,
            env,
            svc.working_dir.as_deref().unwrap_or(""),
            volumes,
            ports,
            is_host_net,
            restart_policy,
            mem_limit,
            cpu_limit,
            svc.oom_score_adj,
            read_only,
            network_name,
            Some(compose_labels),
        )?;

        if let Err(e) = inject_hosts_entries(&data_dir, &container_name, &cf.services, &name) {
            output.push_str(&format!("  ⚠ Warning: could not inject hosts: {}\n", e));
        }

        output.push_str(&format!("  ▶ Starting container '{}'...\n", container_name));
        container_start(&container_name)?;

        output.push_str(&format!("  ✓ Service '{}' is up.\n", name));
    }

    Ok(output)
}

pub fn compose_stop(path: &str) -> Result<String, String> {
    let f = File::open(path).map_err(|e| format!("Failed to read compose file: {}", e))?;
    let cf: ComposeFile = serde_yaml::from_reader(f).map_err(|e| format!("Failed to parse YAML: {}", e))?;

    let ordered = order_services(&cf.services);
    let mut output = String::new();

    for name in ordered.into_iter().rev() {
        let svc = &cf.services[&name];
        let loaded_env = load_env_file(path, svc.env_file.as_ref());
        let raw_container_name = svc.container_name.as_ref().unwrap_or(&name);
        let container_name_expanded = expand_env_vars(raw_container_name, &loaded_env);
        let container_name = if container_name_expanded.is_empty() { name.clone() } else { container_name_expanded };

        output.push_str(&format!("▶ Stopping service '{}' (container: {})...\n", name, container_name));
        let _ = container_stop(&container_name);
    }

    Ok(output)
}

pub fn compose_down(path: &str) -> Result<String, String> {
    let f = File::open(path).map_err(|e| format!("Failed to read compose file: {}", e))?;
    let cf: ComposeFile = serde_yaml::from_reader(f).map_err(|e| format!("Failed to parse YAML: {}", e))?;

    let ordered = order_services(&cf.services);
    let mut output = String::new();

    for name in ordered.into_iter().rev() {
        let svc = &cf.services[&name];
        let loaded_env = load_env_file(path, svc.env_file.as_ref());
        let raw_container_name = svc.container_name.as_ref().unwrap_or(&name);
        let container_name_expanded = expand_env_vars(raw_container_name, &loaded_env);
        let container_name = if container_name_expanded.is_empty() { name.clone() } else { container_name_expanded };

        output.push_str(&format!("▶ Stopping service '{}' (container: {})...\n", name, container_name));
        let _ = container_stop(&container_name);
        output.push_str(&format!("▶ Removing container '{}'...\n", container_name));
        let _ = container_delete(&container_name);
    }

    Ok(output)
}
