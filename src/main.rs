use clap::{Parser, Subcommand};
use std::collections::HashMap;

#[derive(Parser)]
#[command(name = "zenobox")]
#[command(author = "NextCore <github.com/nextcore>")]
#[command(version = "0.2.1")]
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
        /// Container ID or name
        container: String,
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
    /// Build, (re)create, start, and attach to containers for a service
    Up {
        /// Path to docker-compose.yml file (default "docker-compose.yml")
        #[arg(short = 'f', long = "file", default_value = "docker-compose.yml")]
        file: String,

        /// Detached mode: Run containers in the background
        #[arg(short = 'd', long = "detach")]
        detach: bool,
    },
    /// Stop and remove containers, networks created by up
    Down {
        /// Path to docker-compose.yml file (default "docker-compose.yml")
        #[arg(short = 'f', long = "file", default_value = "docker-compose.yml")]
        file: String,
    },
    /// List containers in compose project
    Ps {
        /// Path to docker-compose.yml file (default "docker-compose.yml")
        #[arg(short = 'f', long = "file", default_value = "docker-compose.yml")]
        file: String,
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
        Commands::Logs { container } => {
            match zenobox::container_logs(&container) {
                Ok(logs) => print!("{}", logs),
                Err(e) => eprintln!("❌ Error fetching logs: {}", e),
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
            ComposeCommands::Up { file, detach: _ } => {
                println!("🐳 [zenobox] Running Docker Compose Up from '{}'...", file);
                let out = zenobox::compose_up(&file)?;
                print!("{}", out);
                println!("✓ Compose Up complete.");
            }
            ComposeCommands::Down { file } => {
                println!("🛑 [zenobox] Running Docker Compose Down from '{}'...", file);
                let out = zenobox::compose_down(&file)?;
                print!("{}", out);
                println!("✓ Compose Down complete.");
            }
            ComposeCommands::Ps { file: _ } => {
                let data_dir = zenobox::get_data_dir();
                let containers = zenobox::container_list_internal(&data_dir, false)?;
                println!("{:<16} {:<24} {:<12} {:<10}", "CONTAINER ID", "IMAGE", "STATUS", "PID");
                println!("{}", "-".repeat(65));
                for c in containers {
                    println!("{:<16} {:<24} {:<12} {:<10}", c.id, c.image, c.status, c.pid);
                }
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
