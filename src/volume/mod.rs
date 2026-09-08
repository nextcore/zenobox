use std::path::{Path, PathBuf};
use std::fs;
use serde::{Serialize, Deserialize};
use crate::utils::get_data_dir;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VolumeInfo {
    pub name: String,
    pub driver: String,
    pub mountpoint: String,
}

pub fn list_volumes() -> Vec<VolumeInfo> {
    let data_dir = get_data_dir();
    let volumes_dir = Path::new(&data_dir).join("volumes");
    let _ = fs::create_dir_all(&volumes_dir);

    let mut list = Vec::new();
    if let Ok(entries) = fs::read_dir(volumes_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                if entry.path().is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    list.push(VolumeInfo {
                        name,
                        driver: "local".to_string(),
                        mountpoint: entry.path().to_string_lossy().to_string(),
                    });
                }
            }
        }
    }
    list
}

pub fn create_volume(name: &str) -> Result<PathBuf, String> {
    let data_dir = get_data_dir();
    let vol_p = Path::new(&data_dir).join("volumes").join(name);
    fs::create_dir_all(&vol_p).map_err(|e| e.to_string())?;
    Ok(vol_p)
}

pub fn delete_volume(name: &str) -> Result<(), String> {
    let data_dir = get_data_dir();
    let vol_p = Path::new(&data_dir).join("volumes").join(name);
    if vol_p.exists() {
        fs::remove_dir_all(vol_p).map_err(|e| e.to_string())?;
    }
    Ok(())
}
