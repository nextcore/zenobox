use std::path::Path;
use std::fs;

pub const EMBEDDED_ROUTES_ZL: &str = include_str!("../../zsrc/routes.zl");
pub const EMBEDDED_DOCKER_API_ZL: &str = include_str!("../../zsrc/docker_api.zl");
pub const EMBEDDED_NATIVE_API_ZL: &str = include_str!("../../zsrc/native_api.zl");

pub struct ZlScriptLoader;

impl ZlScriptLoader {
    pub fn load_script(rel_path: &str) -> String {
        let dev_path = Path::new("./zsrc").join(rel_path);
        if dev_path.exists() {
            if let Ok(content) = fs::read_to_string(&dev_path) {
                return content;
            }
        }
        let dev_path_etc = Path::new("/etc/zenobox/zsrc").join(rel_path);
        if dev_path_etc.exists() {
            if let Ok(content) = fs::read_to_string(&dev_path_etc) {
                return content;
            }
        }

        match rel_path {
            "routes.zl" => EMBEDDED_ROUTES_ZL.to_string(),
            "docker_api.zl" => EMBEDDED_DOCKER_API_ZL.to_string(),
            "native_api.zl" => EMBEDDED_NATIVE_API_ZL.to_string(),
            _ => String::new(),
        }
    }

    pub fn is_dev_mode() -> bool {
        Path::new("./zsrc").exists() || Path::new("/etc/zenobox/zsrc").exists()
    }
}

use zenocore::Engine;
use super::slots::register_slots;

pub fn create_engine() -> Engine {
    let mut engine = Engine::new();
    register_slots(&mut engine);
    engine
}
