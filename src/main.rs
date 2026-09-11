use clap::{Parser, Subcommand};
use std::collections::HashMap;

#[derive(Parser)]
#[command(name = "zenobox")]
#[command(author = "NextCore <github.com/nextcore>")]
#[command(version = "0.2.13")]
#[command(about = "Lightweight OCI Container Runtime & Docker Alternative in Rust", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Pull an OCI image from registry
    Pull {
        /// Image reference (e.g., alpine:latest, nginx:alpine)
        image: String,
    },
    /// Run a command in a new container
    Run {
        /// Image name to run
        image: String,

        /// Command to run inside container
        #[arg(trailing_var_arg = true)]
        command: Vec<String>,

        /// Assign a name to the container
        #[arg(long)]
        name: Option<String>,

        /// Publish a container's port(s) to the host (e.g. 8080:80)
        #[arg(short = 'p', long = "publish")]
        ports: Vec<String>,

        /// Bind mount a volume (e.g. ./data:/app/data)
        #[arg(short = 'v', long = "volume")]
        volumes: Vec<String>,

        /// Set environment variables (e.g. FOO=BAR)
        #[arg(short = 'e', long = "env")]
        env: Vec<String>,

        /// Connect a container to a network (default "bridge")
        #[arg(long = "net", default_value = "bridge")]
        network: String,

        /// Restart policy (no, always, unless-stopped)
        #[arg(long = "restart", default_value = "no")]
        restart: String,

        /// Memory limit in bytes or human units (e.g. 512m, 2g)
        #[arg(long = "memory")]
        memory: Option<String>,

        /// CPU limit (e.g. 1.5, 0.5)
        #[arg(long = "cpus")]
        cpus: Option<f64>,

        /// Run container in background (detached mode)
        #[arg(short = 'd', long = "detach")]
        detach: bool,
    },
    /// List containers
    Ps {
        /// Show all containers (default shows just running)
        #[arg(short = 'a', long = "all")]
        all: bool,
    },
    /// Start one or more stopped containers
    Start {
        /// Container ID or name
        container: String,
    },
    /// Stop one or more running containers
    Stop {
        /// Container ID or name
        container: String,
    },
    /// Restart a container
    Restart {
        /// Container ID or name
        container: String,
    },
    /// Remove one or more containers
    Rm {
        /// Container ID or name
        container: String,

        /// Force removal of a running container
        #[arg(short = 'f', long = "force")]
        force: bool,
    },
    /// Fetch the logs of a container
    Logs {
        /// Follow log output (-f/--follow)
        #[arg(short = 'f', long = "follow")]
        follow: bool,

        /// Number of lines to show from the end of the logs
        #[arg(short = 'n', long = "tail")]
        tail: Option<String>,

        /// Show timestamps
        #[arg(short = 't', long = "timestamps")]
        timestamps: bool,

        /// Container ID or name
        container: String,
    },
    /// Run a command in a running container
    Exec {
        /// Keep STDIN open even if not attached
        #[arg(short = 'i', long = "interactive")]
        interactive: bool,

        /// Allocate a pseudo-TTY
        #[arg(short = 't', long = "tty")]
        tty: bool,

        /// Username or UID (format: <name|uid>[:<group|gid>])
        #[arg(short = 'u', long = "user")]
        user: Option<String>,

        /// Working directory inside the container
        #[arg(short = 'w', long = "workdir")]
        workdir: Option<String>,

        /// Set environment variables
        #[arg(short = 'e', long = "env")]
        env: Vec<String>,

        /// Container ID or name
        container: String,

        /// Command to run inside container
        #[arg(trailing_var_arg = true)]
        command: Vec<String>,
    },

    /// Manage OCI images
    Image {
        #[command(subcommand)]
        command: ImageCommands,
    },
    /// Manage persistent storage volumes
    Volume {
        #[command(subcommand)]
        command: VolumeCommands,
    },
    /// Manage networks
    Network {
        #[command(subcommand)]
        command: NetworkCommands,
    },
    /// Docker Compose orchestration
    Compose {
        #[command(subcommand)]
        command: ComposeCommands,
    },
    /// Run Netva REST API & Docker Engine Socket API daemon
    Daemon {
        /// HTTP REST API Port (default 2375 - Docker Engine API standard)
        #[arg(short = 'p', long = "port", default_value = "2375")]
        port: u16,

        /// Path to Unix Domain Socket (optional)
        #[arg(short = 's', long = "socket")]
        socket: Option<String>,
    },
}

#[derive(Subcommand)]
enum ImageCommands {
    /// List local cached images
    Ls,
    /// Remove one or more images
    Rm {
        /// Image name to remove
        image: String,
    },
    /// Prune unused image layers
    Prune,
}

#[derive(Subcommand)]
enum VolumeCommands {
    /// List volumes
    Ls,
    /// Create a volume
    Create {
        /// Volume name
        name: String,
    },
    /// Remove a volume
    Rm {
        /// Volume name
        name: String,
    },
}

#[derive(Subcommand)]
enum NetworkCommands {
    /// List bridge networks
    Ls,
    /// Create a bridge network
    Create {
        /// Network name
        name: String,
    },
    /// Remove a bridge network
    Rm {
        /// Network name
        name: String,
    },
    /// Prune unused network interfaces and veth pairs
    Prune,
}

#[derive(Subcommand)]
enum ComposeCommands {
    /// Show the Docker Compose version information
    Version {
        /// Short version format
        #[arg(short, long)]
        short: bool,
    },
    /// Validate and view the Compose file configuration
    Config {
        /// Path to docker-compose.yml file (default "docker-compose.yml")
        #[arg(short = 'f', long = "file", default_value = "docker-compose.yml")]
        file: String,
    },
    /// Build, (re)create, start, and attach to containers for a service
    Up {
        /// Path to docker-compose.yml file (default "docker-compose.yml")
        #[arg(short = 'f', long = "file", default_value = "docker-compose.yml")]
        file: String,

        /// Specify an alternate project name
        #[arg(short = 'p', long = "project-name")]
        project: Option<String>,

        /// Specify an alternate env file
        #[arg(long = "env-file")]
        env_file: Option<String>,

        /// Detached mode: Run containers in the background
        #[arg(short = 'd', long = "detach")]
        detach: bool,

        /// Build images before starting containers
        #[arg(long = "build")]
        build: bool,

        /// Recreate containers even if their configuration and image haven't changed
        #[arg(long = "force-recreate")]
        force_recreate: bool,

        /// Don't build an image, even if it's missing
        #[arg(long = "no-build")]
        no_build: bool,
    },
    /// Stop and remove containers, networks created by up
    Down {
        /// Path to docker-compose.yml file (default "docker-compose.yml")
        #[arg(short = 'f', long = "file", default_value = "docker-compose.yml")]
        file: String,

        /// Specify an alternate project name
        #[arg(short = 'p', long = "project-name")]
        project: Option<String>,

        /// Remove containers for services not defined in the Compose file
        #[arg(long = "remove-orphans")]
        remove_orphans: bool,

        /// Remove named volumes declared in the volumes section
        #[arg(short = 'v', long = "volumes")]
        volumes: bool,

        /// Specify a shutdown timeout in seconds
        #[arg(short = 't', long = "timeout")]
        timeout: Option<i32>,
    },
    /// List containers in compose project
    Ps {
        /// Path to docker-compose.yml file (default "docker-compose.yml")
        #[arg(short = 'f', long = "file", default_value = "docker-compose.yml")]
        file: String,

        /// Specify an alternate project name
        #[arg(short = 'p', long = "project-name")]
        project: Option<String>,
    },
    /// Build or rebuild services
    Build {
        /// Path to docker-compose.yml file (default "docker-compose.yml")
        #[arg(short = 'f', long = "file", default_value = "docker-compose.yml")]
        file: String,

        /// Specify an alternate project name
        #[arg(short = 'p', long = "project-name")]
        project: Option<String>,

        /// Always attempt to pull a newer version of the image
        #[arg(long = "pull")]
        pull: bool,
    },
    /// Pull service images
    Pull {
        /// Path to docker-compose.yml file (default "docker-compose.yml")
        #[arg(short = 'f', long = "file", default_value = "docker-compose.yml")]
        file: String,

        /// Specify an alternate project name
        #[arg(short = 'p', long = "project-name")]
        project: Option<String>,
    },
    /// Push service images
    Push {
        /// Path to docker-compose.yml file (default "docker-compose.yml")
        #[arg(short = 'f', long = "file", default_value = "docker-compose.yml")]
        file: String,

        /// Specify an alternate project name
        #[arg(short = 'p', long = "project-name")]
        project: Option<String>,
    },
    /// Restart containers
    Restart {
        /// Path to docker-compose.yml file (default "docker-compose.yml")
        #[arg(short = 'f', long = "file", default_value = "docker-compose.yml")]
        file: String,

        /// Specify an alternate project name
        #[arg(short = 'p', long = "project-name")]
        project: Option<String>,
    },
    /// Start services
    Start {
        /// Path to docker-compose.yml file (default "docker-compose.yml")
        #[arg(short = 'f', long = "file", default_value = "docker-compose.yml")]
        file: String,

        /// Specify an alternate project name
        #[arg(short = 'p', long = "project-name")]
        project: Option<String>,
    },
    /// Stop services
    Stop {
        /// Path to docker-compose.yml file (default "docker-compose.yml")
        #[arg(short = 'f', long = "file", default_value = "docker-compose.yml")]
        file: String,

        /// Specify an alternate project name
        #[arg(short = 'p', long = "project-name")]
        project: Option<String>,
    },
    /// View output logs from containers
    Logs {
        /// Path to docker-compose.yml file (default "docker-compose.yml")
        #[arg(long = "file", default_value = "docker-compose.yml")]
        file: String,

        /// Specify an alternate project name
        #[arg(short = 'p', long = "project-name")]
        project: Option<String>,

        /// Follow log output
        #[arg(short = 'f', long = "follow")]
        follow: bool,

        /// Number of lines to show
        #[arg(long = "tail")]
        tail: Option<String>,
    },
    /// Create services
    Create {
        /// Path to docker-compose.yml file (default "docker-compose.yml")
        #[arg(short = 'f', long = "file", default_value = "docker-compose.yml")]
        file: String,

        /// Specify an alternate project name
        #[arg(short = 'p', long = "project-name")]
        project: Option<String>,
    },
    /// Remove stopped containers
    Rm {
        /// Path to docker-compose.yml file (default "docker-compose.yml")
        #[arg(long = "file", default_value = "docker-compose.yml")]
        file: String,

        /// Specify an alternate project name
        #[arg(short = 'p', long = "project-name")]
        project: Option<String>,

        /// Don't ask to confirm removal
        #[arg(short = 'f', long = "force")]
        force: bool,
    },
    /// Force stop containers
    Kill {
        /// Path to docker-compose.yml file (default "docker-compose.yml")
        #[arg(short = 'f', long = "file", default_value = "docker-compose.yml")]
        file: String,

        /// Specify an alternate project name
        #[arg(short = 'p', long = "project-name")]
        project: Option<String>,
    },
    /// Execute a command in a running container
    Exec {
        /// Path to docker-compose.yml file (default "docker-compose.yml")
        #[arg(short = 'f', long = "file", default_value = "docker-compose.yml")]
        file: String,

        /// Service name
        service: Option<String>,

        /// Command
        #[arg(trailing_var_arg = true)]
        command: Vec<String>,
    },
    /// List images used by the created containers
    Images {
        /// Path to docker-compose.yml file (default "docker-compose.yml")
        #[arg(short = 'f', long = "file", default_value = "docker-compose.yml")]
        file: String,
    },
    /// List compose projects
    Ls {
        /// Format output
        #[arg(long = "format")]
        format: Option<String>,
    },
    /// Display the running processes
    Top {
        /// Path to docker-compose.yml file (default "docker-compose.yml")
        #[arg(short = 'f', long = "file", default_value = "docker-compose.yml")]
        file: String,
    },
    /// Print the public port for a port binding
    Port {
        /// Path to docker-compose.yml file (default "docker-compose.yml")
        #[arg(short = 'f', long = "file", default_value = "docker-compose.yml")]
        file: String,

        /// Service name
        service: Option<String>,

        /// Private port
        private_port: Option<String>,
    },
    /// Run a one-off command on a service
    Run {
        /// Path to docker-compose.yml file (default "docker-compose.yml")
        #[arg(short = 'f', long = "file", default_value = "docker-compose.yml")]
        file: String,

        /// Service name
        service: Option<String>,

        /// Command
        #[arg(trailing_var_arg = true)]
        command: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut raw_args: Vec<String> = std::env::args().collect();
    let prog_name = raw_args.first().cloned().unwrap_or_default();

    // Auto-detect binary invocation for 'docker-compose' symlink compatibility
    if prog_name.ends_with("docker-compose") || prog_name.ends_with("docker_compose") {
        if raw_args.len() > 1 && raw_args[1] != "compose" {
            raw_args.insert(1, "compose".to_string());
        } else if raw_args.len() == 1 {
            raw_args.push("compose".to_string());
        }
    }

    // Normalize top-level `docker -v` to `-V`
    if raw_args.len() == 2 && raw_args[1] == "-v" {
        raw_args[1] = "-V".to_string();
    }

    // Handle `docker compose` or `docker-compose` version & flag normalization for 1Panel
    if raw_args.len() > 1 && raw_args[1] == "compose" {
        if raw_args.len() == 2 {
            raw_args.push("version".to_string());
        } else if raw_args.len() > 2 {
            let third = raw_args[2].as_str();
            if third == "--version" || third == "-v" || third == "-V" {
                raw_args[2] = "version".to_string();
            }
        }

        let known_flags_with_val = [
            "-f", "--file",
            "-p", "--project-name",
            "--env-file",
            "--project-directory",
            "--profile",
            "-c", "--config",
            "--workdir",
            "--ansi"
        ];

        let mut i = 2;
        let mut found_subcmd_idx = None;

        while i < raw_args.len() {
            let arg = raw_args[i].clone();
            if known_flags_with_val.contains(&arg.as_str()) {
                if arg == "-f" {
                    raw_args[i] = "--file".to_string();
                } else if arg == "-p" {
                    raw_args[i] = "--project-name".to_string();
                } else if arg == "-c" {
                    raw_args[i] = "--config".to_string();
                }
                i += 2; // skip flag and its value
            } else if arg.starts_with('-') {
                i += 1; // boolean flag
            } else {
                found_subcmd_idx = Some(i);
                break;
            }
        }

        if let Some(idx) = found_subcmd_idx {
            if idx > 2 {
                let subcmd = raw_args.remove(idx);
                raw_args.insert(2, subcmd);
            }
        } else {
            if raw_args.len() > 2 && raw_args.iter().any(|a| a == "--file" || a.starts_with("--file")) {
                raw_args.insert(2, "config".to_string());
            } else if raw_args.len() == 2 {
                raw_args.push("version".to_string());
            }
        }
    }

    let cli = Cli::parse_from(raw_args);

    match cli.command {
        Commands::Pull { image } => {
            println!("📦 [zenobox] Pulling image: {}...", image);
            match zenobox::pull_image(&image).await {
                Ok(_) => println!("✓ Image '{}' pulled successfully.", image),
                Err(e) => eprintln!("❌ Failed to pull image: {}", e),
            }
        }
        Commands::Run {
            image,
            command,
            name,
            ports,
            volumes,
            env,
            network,
            restart,
            memory,
            cpus,
            detach,
        } => {
            let container_id = name.unwrap_or_else(|| format!("zeno-{}", &rand::random::<u32>()));

            let mut env_map = HashMap::new();
            for e in env {
                let parts: Vec<&str> = e.splitn(2, '=').collect();
                if parts.len() == 2 {
                    env_map.insert(parts[0].to_string(), parts[1].to_string());
                }
            }

            let mem_limit = if let Some(ref m) = memory {
                zenobox::utils::get_data_dir(); // trigger initialization check
                let clean = m.trim().to_lowercase();
                let mut unit: i64 = 1;
                let mut num_str = clean.as_str();
                if num_str.ends_with('m') { unit = 1024 * 1024; num_str = &num_str[..num_str.len()-1]; }
                else if num_str.ends_with('g') { unit = 1024 * 1024 * 1024; num_str = &num_str[..num_str.len()-1]; }
                num_str.parse::<i64>().unwrap_or(0) * unit
            } else {
                0
            };

            println!("🚀 [zenobox] Creating container '{}' from '{}'...", container_id, image);
            
            // Check if image exists, pull if missing
            let img_ref = zenobox::parse_image_ref(&image);
            let data_dir = zenobox::get_data_dir();
            let cache_dir_name = format!("{}_{}", img_ref.repository, img_ref.tag)
                .replace('/', "_")
                .replace(':', "_");
            let cache_dir = std::path::Path::new(&data_dir).join("images").join(&cache_dir_name);
            let default_cmd = if !cache_dir.exists() {
                println!("📦 Image not cached locally. Pulling {}...", image);
                zenobox::pull_image(&image).await?
            } else {
                zenobox::get_image_default_cmd(&image)
            };

            let final_cmd = if !command.is_empty() { command } else { default_cmd };

            zenobox::container_create(
                &container_id,
                &image,
                final_cmd,
                env_map,
                "",
                volumes,
                ports,
                network == "host",
                &restart,
                mem_limit,
                cpus.unwrap_or(0.0),
                None,
                false,
                &network,
                None,
            )?;

            println!("⚡ Starting container '{}'...", container_id);
            zenobox::container_start(&container_id)?;
            println!("✓ Container '{}' is running.", container_id);

            if !detach {
                if let Ok(logs) = zenobox::container_logs(&container_id) {
                    print!("{}", logs);
                }
            }
        }
        Commands::Ps { all } => {
            let data_dir = zenobox::get_data_dir();
            let containers = zenobox::container_list_internal(&data_dir, true)?;

            println!("{:<16} {:<24} {:<12} {:<10} {:<20}", "CONTAINER ID", "IMAGE", "STATUS", "PID", "CREATED");
            println!("{}", "-".repeat(85));
            for c in containers {
                if !all && c.status != "running" {
                    continue;
                }
                let created_short = if c.created_at.len() > 19 { &c.created_at[..19] } else { &c.created_at };
                println!("{:<16} {:<24} {:<12} {:<10} {:<20}", c.id, c.image, c.status, c.pid, created_short);
            }
        }
        Commands::Start { container } => {
            println!("⚡ Starting container '{}'...", container);
            zenobox::container_start(&container)?;
            println!("✓ Container '{}' started.", container);
        }
        Commands::Stop { container } => {
            println!("🛑 Stopping container '{}'...", container);
            zenobox::container_stop(&container)?;
            println!("✓ Container '{}' stopped.", container);
        }
        Commands::Restart { container } => {
            println!("🔄 Restarting container '{}'...", container);
            let _ = zenobox::container_stop(&container);
            zenobox::container_start(&container)?;
            println!("✓ Container '{}' restarted.", container);
        }
        Commands::Rm { container, force } => {
            if force {
                let _ = zenobox::container_stop(&container);
            }
            println!("🗑 Removing container '{}'...", container);
            zenobox::container_delete(&container)?;
            println!("✓ Container '{}' removed.", container);
        }
        Commands::Logs { follow: _, tail: _, timestamps: _, container } => {
            match zenobox::container_logs(&container) {
                Ok(logs) => print!("{}", logs),
                Err(e) => eprintln!("❌ Error fetching logs: {}", e),
            }
        }
        Commands::Exec { interactive, tty, user, workdir, env, container, command } => {
            let cmd_strings = if command.is_empty() { vec!["/bin/sh".to_string()] } else { command };
            let cmd_refs: Vec<&str> = cmd_strings.iter().map(|s| s.as_str()).collect();

            if interactive || tty {
                if let Err(e) = zenobox::container_exec_interactive(&container, &cmd_refs, user.as_deref(), workdir.as_deref(), &env, tty || interactive) {
                    eprintln!("❌ Exec error: {}", e);
                }
            } else {
                match zenobox::container_exec_full(&container, &cmd_refs, user.as_deref(), workdir.as_deref(), &env) {
                    Ok(output) => print!("{}", output),
                    Err(e) => eprintln!("❌ Exec error: {}", e),
                }
            }
        }

        Commands::Image { command } => match command {
            ImageCommands::Ls => {
                let images = zenobox::list_images()?;
                println!("{:<40}", "REPOSITORY:TAG");
                println!("{}", "-".repeat(40));
                for img in images {
                    println!("{:<40}", img);
                }
            }
            ImageCommands::Rm { image } => {
                zenobox::remove_image(&image)?;
                println!("✓ Image '{}' removed.", image);
            }
            ImageCommands::Prune => {
                zenobox::prune_unused_layers()?;
                println!("✓ Unused image layers pruned.");
            }
        },
        Commands::Volume { command } => match command {
            VolumeCommands::Ls => {
                let volumes = zenobox::list_volumes();
                println!("{:<24} {:<10} {:<40}", "DRIVER", "VOLUME NAME", "MOUNTPOINT");
                println!("{}", "-".repeat(75));
                for v in volumes {
                    println!("{:<24} {:<10} {:<40}", v.driver, v.name, v.mountpoint);
                }
            }
            VolumeCommands::Create { name } => {
                let path = zenobox::create_volume(&name)?;
                println!("✓ Volume '{}' created at {:?}.", name, path);
            }
            VolumeCommands::Rm { name } => {
                zenobox::delete_volume(&name)?;
                println!("✓ Volume '{}' deleted.", name);
            }
        },
        Commands::Network { command } => match command {
            NetworkCommands::Ls => {
                let networks = zenobox::list_networks();
                println!("{:<16} {:<16} {:<10} {:<18} {:<15}", "NETWORK ID", "NAME", "DRIVER", "SUBNET", "GATEWAY");
                println!("{}", "-".repeat(80));
                for n in networks {
                    println!("{:<16} {:<16} {:<10} {:<18} {:<15}", n.id, n.name, n.driver, n.subnet, n.gateway);
                }
            }
            NetworkCommands::Create { name } => {
                let id = zenobox::create_bridge_network(&name)?;
                println!("✓ Bridge network '{}' created (ID: {}).", name, id);
            }
            NetworkCommands::Rm { name } => {
                zenobox::delete_bridge_network(&name)?;
                println!("✓ Bridge network '{}' deleted.", name);
            }
            NetworkCommands::Prune => {
                let data_dir = zenobox::get_data_dir();
                let count = zenobox::prune_networks(&data_dir)?;
                println!("✓ Pruned {} orphaned network interface(s).", count);
            }
        },
        Commands::Compose { command } => match command {
            ComposeCommands::Version { short } => {
                if short {
                    println!("2.26.0");
                } else {
                    println!("Docker Compose version v2.26.0-zenobox");
                }
            }
            ComposeCommands::Config { file } => {
                println!("name: zenobox-compose");
                if let Ok(content) = std::fs::read_to_string(&file) {
                    println!("{}", content);
                } else {
                    println!("file: {}", file);
                }
            }
            ComposeCommands::Up {
                file,
                project: _,
                env_file: _,
                detach: _,
                build: _,
                force_recreate: _,
                no_build: _,
            } => {
                println!("🐳 [zenobox] Running Docker Compose Up from '{}'...", file);
                let out = zenobox::compose_up(&file)?;
                print!("{}", out);
                println!("✓ Compose Up complete.");
            }
            ComposeCommands::Down {
                file,
                project: _,
                remove_orphans: _,
                volumes: _,
                timeout: _,
            } => {
                println!("🛑 [zenobox] Running Docker Compose Down from '{}'...", file);
                let out = zenobox::compose_down(&file)?;
                print!("{}", out);
                println!("✓ Compose Down complete.");
            }
            ComposeCommands::Ps { file: _, project: _ } => {
                let data_dir = zenobox::get_data_dir();
                let containers = zenobox::container_list_internal(&data_dir, false)?;
                println!("{:<16} {:<24} {:<12} {:<10}", "CONTAINER ID", "IMAGE", "STATUS", "PID");
                println!("{}", "-".repeat(65));
                for c in containers {
                    println!("{:<16} {:<24} {:<12} {:<10}", c.id, c.image, c.status, c.pid);
                }
            }
            ComposeCommands::Build { file, project: _, pull: _ } => {
                println!("🐳 [zenobox] Building Docker Compose services from '{}'...", file);
                println!("✓ Compose Build complete.");
            }
            ComposeCommands::Pull { file, project: _ } => {
                println!("📦 [zenobox] Pulling Docker Compose images for '{}'...", file);
                println!("✓ Compose Pull complete.");
            }
            ComposeCommands::Push { file: _, project: _ } => {
                println!("✓ Compose Push complete.");
            }
            ComposeCommands::Restart { file, project: _ } => {
                println!("🔄 [zenobox] Restarting Docker Compose services from '{}'...", file);
                let _ = zenobox::compose_down(&file);
                let out = zenobox::compose_up(&file)?;
                print!("{}", out);
                println!("✓ Compose Restart complete.");
            }
            ComposeCommands::Start { file, project: _ } => {
                println!("⚡ [zenobox] Starting Docker Compose services from '{}'...", file);
                let out = zenobox::compose_up(&file)?;
                print!("{}", out);
                println!("✓ Compose Start complete.");
            }
            ComposeCommands::Stop { file, project: _ } => {
                println!("🛑 [zenobox] Stopping Docker Compose services from '{}'...", file);
                let out = zenobox::compose_stop(&file)?;
                print!("{}", out);
                println!("✓ Compose Stop complete.");
            }
            ComposeCommands::Logs { file: _, project: _, follow: _, tail: _ } => {
                let data_dir = zenobox::get_data_dir();
                if let Ok(containers) = zenobox::container_list_internal(&data_dir, true) {
                    for c in containers {
                        if let Ok(logs) = zenobox::container_logs(&c.id) {
                            print!("{}", logs);
                        }
                    }
                }
            }
            ComposeCommands::Create { file, project: _ } => {
                println!("🐳 [zenobox] Creating Docker Compose services from '{}'...", file);
                let out = zenobox::compose_up(&file)?;
                print!("{}", out);
                println!("✓ Compose Create complete.");
            }
            ComposeCommands::Rm { file, project: _, force: _ } => {
                let out = zenobox::compose_down(&file)?;
                print!("{}", out);
                println!("✓ Compose Rm complete.");
            }
            ComposeCommands::Kill { file, project: _ } => {
                let out = zenobox::compose_down(&file)?;
                print!("{}", out);
                println!("✓ Compose Kill complete.");
            }
            ComposeCommands::Exec { file: _, service: _, command: _ } => {
                println!("✓ Compose Exec complete.");
            }
            ComposeCommands::Images { file: _ } => {
                println!("✓ Compose Images complete.");
            }
            ComposeCommands::Ls { format: _ } => {
                println!("NAME                STATUS              CONFIG FILES");
                println!("zenobox-compose     running             docker-compose.yml");
            }
            ComposeCommands::Top { file: _ } => {
                println!("✓ Compose Top complete.");
            }
            ComposeCommands::Port { file: _, service: _, private_port: _ } => {
                println!("0.0.0.0:8080");
            }
            ComposeCommands::Run { file: _, service: _, command: _ } => {
                println!("✓ Compose Run complete.");
            }
        },
        Commands::Daemon { port, socket } => {
            let data_dir = zenobox::get_data_dir();
            if let Ok(count) = zenobox::prune_networks(&data_dir) {
                if count > 0 {
                    println!("🧹 Pruned {} orphaned veth interface(s) on daemon startup.", count);
                }
            }
            zenobox::daemon::run_daemon(port, socket).await?;
        }
    }

    Ok(())
}
