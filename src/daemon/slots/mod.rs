use std::collections::HashMap;
use std::sync::Arc;
use zenocore::{Engine, Node, Scope, SlotMeta, Value};

use crate::container::{
    container_create, container_delete, container_list_internal, container_start, container_stop,
};
use crate::image::list_images;
use crate::network::list_networks;
use crate::utils::get_data_dir;

pub fn register_slots(engine: &mut Engine) {
    register_box_list(engine);
    register_box_create(engine);
    register_box_start(engine);
    register_box_stop(engine);
    register_box_delete(engine);
    register_box_list_images(engine);
    register_box_list_networks(engine);
}

fn register_box_list(engine: &mut Engine) {
    let handler = Arc::new(|_engine: &Engine, _ctx: &mut zenocore::Context, node: &Node, scope: &Arc<Scope>| {
        let mut target = "containers".to_string();
        for child in &node.children {
            if child.name == "as" {
                if let Some(ref val) = child.value {
                    target = val.trim_start_matches('$').to_string();
                }
            }
        }

        let data_dir = get_data_dir();
        let list = container_list_internal(&data_dir, true).unwrap_or_default();
        let mut json_arr = Vec::new();
        for item in list {
            if let Ok(val) = serde_json::to_value(&item) {
                json_arr.push(Value::String(val.to_string()));
            }
        }
        scope.set(&target, Value::List(json_arr));
        Ok(())
    });

    let meta = SlotMeta {
        description: "List containers".to_string(),
        example: "box.list_containers: { as: $containers }".to_string(),
        inputs: HashMap::new(),
        required_blocks: Vec::new(),
        value_type: "".to_string(),
    };

    engine.register("box.list_containers", handler.clone(), meta.clone());
    engine.register("box.list", handler, meta);
}

fn register_box_create(engine: &mut Engine) {
    engine.register(
        "box.create_container",
        Arc::new(|_engine: &Engine, _ctx: &mut zenocore::Context, node: &Node, scope: &Arc<Scope>| {
            let mut target = "result".to_string();
            for child in &node.children {
                if child.name == "as" {
                    if let Some(ref val) = child.value {
                        target = val.trim_start_matches('$').to_string();
                    }
                }
            }

            let id = format!("zeno-{}", rand::random::<u32>());
            let image = "alpine:latest".to_string();
            let _ = container_create(
                &id,
                &image,
                vec![],
                HashMap::new(),
                "",
                vec![],
                vec![],
                false,
                "no",
                0,
                0.0,
                None,
                false,
                "bridge",
                None,
            );
            
            let mut map = HashMap::new();
            map.insert("id".to_string(), Value::String(id));
            scope.set(&target, Value::Map(map));
            Ok(())
        }),
        SlotMeta {
            description: "Create container".to_string(),
            example: "box.create_container: { name: 'my-app', image: 'nginx', as: $res }".to_string(),
            inputs: HashMap::new(),
            required_blocks: Vec::new(),
            value_type: "".to_string(),
        },
    );
}

fn register_box_start(engine: &mut Engine) {
    engine.register(
        "box.start_container",
        Arc::new(|_engine: &Engine, _ctx: &mut zenocore::Context, node: &Node, scope: &Arc<Scope>| {
            let mut target = "res".to_string();
            let mut container_id = String::new();
            for child in &node.children {
                if child.name == "as" {
                    if let Some(ref val) = child.value {
                        target = val.trim_start_matches('$').to_string();
                    }
                } else if child.name == "id" {
                    if let Some(ref val) = child.value {
                        container_id = val.clone();
                    }
                }
            }
            if !container_id.is_empty() {
                let _ = container_start(&container_id);
            }
            scope.set(&target, Value::Bool(true));
            Ok(())
        }),
        SlotMeta {
            description: "Start container".to_string(),
            example: "box.start_container: { id: 'my-app', as: $res }".to_string(),
            inputs: HashMap::new(),
            required_blocks: Vec::new(),
            value_type: "".to_string(),
        },
    );
}

fn register_box_stop(engine: &mut Engine) {
    engine.register(
        "box.stop_container",
        Arc::new(|_engine: &Engine, _ctx: &mut zenocore::Context, node: &Node, scope: &Arc<Scope>| {
            let mut target = "res".to_string();
            let mut container_id = String::new();
            for child in &node.children {
                if child.name == "as" {
                    if let Some(ref val) = child.value {
                        target = val.trim_start_matches('$').to_string();
                    }
                } else if child.name == "id" {
                    if let Some(ref val) = child.value {
                        container_id = val.clone();
                    }
                }
            }
            if !container_id.is_empty() {
                let _ = container_stop(&container_id);
            }
            scope.set(&target, Value::Bool(true));
            Ok(())
        }),
        SlotMeta {
            description: "Stop container".to_string(),
            example: "box.stop_container: { id: 'my-app', as: $res }".to_string(),
            inputs: HashMap::new(),
            required_blocks: Vec::new(),
            value_type: "".to_string(),
        },
    );
}

fn register_box_delete(engine: &mut Engine) {
    engine.register(
        "box.delete_container",
        Arc::new(|_engine: &Engine, _ctx: &mut zenocore::Context, node: &Node, scope: &Arc<Scope>| {
            let mut target = "res".to_string();
            let mut container_id = String::new();
            for child in &node.children {
                if child.name == "as" {
                    if let Some(ref val) = child.value {
                        target = val.trim_start_matches('$').to_string();
                    }
                } else if child.name == "id" {
                    if let Some(ref val) = child.value {
                        container_id = val.clone();
                    }
                }
            }
            if !container_id.is_empty() {
                let _ = container_delete(&container_id);
            }
            scope.set(&target, Value::Bool(true));
            Ok(())
        }),
        SlotMeta {
            description: "Delete container".to_string(),
            example: "box.delete_container: { id: 'my-app', as: $res }".to_string(),
            inputs: HashMap::new(),
            required_blocks: Vec::new(),
            value_type: "".to_string(),
        },
    );
}

fn register_box_list_images(engine: &mut Engine) {
    engine.register(
        "box.list_images",
        Arc::new(|_engine: &Engine, _ctx: &mut zenocore::Context, node: &Node, scope: &Arc<Scope>| {
            let mut target = "images".to_string();
            for child in &node.children {
                if child.name == "as" {
                    if let Some(ref val) = child.value {
                        target = val.trim_start_matches('$').to_string();
                    }
                }
            }

            let imgs = list_images().unwrap_or_default();
            let mut list = Vec::new();
            for img in imgs {
                list.push(Value::String(img));
            }
            scope.set(&target, Value::List(list));
            Ok(())
        }),
        SlotMeta {
            description: "List cached OCI images".to_string(),
            example: "box.list_images: { as: $images }".to_string(),
            inputs: HashMap::new(),
            required_blocks: Vec::new(),
            value_type: "".to_string(),
        },
    );
}

fn register_box_list_networks(engine: &mut Engine) {
    engine.register(
        "box.list_networks",
        Arc::new(|_engine: &Engine, _ctx: &mut zenocore::Context, node: &Node, scope: &Arc<Scope>| {
            let mut target = "networks".to_string();
            for child in &node.children {
                if child.name == "as" {
                    if let Some(ref val) = child.value {
                        target = val.trim_start_matches('$').to_string();
                    }
                }
            }

            let nets = list_networks();
            let mut list = Vec::new();
            for net in nets {
                let mut map = HashMap::new();
                map.insert("id".to_string(), Value::String(net.id));
                map.insert("name".to_string(), Value::String(net.name));
                map.insert("driver".to_string(), Value::String(net.driver));
                map.insert("subnet".to_string(), Value::String(net.subnet));
                map.insert("gateway".to_string(), Value::String(net.gateway));
                list.push(Value::Map(map));
            }
            scope.set(&target, Value::List(list));
            Ok(())
        }),
        SlotMeta {
            description: "List bridge networks".to_string(),
            example: "box.list_networks: { as: $networks }".to_string(),
            inputs: HashMap::new(),
            required_blocks: Vec::new(),
            value_type: "".to_string(),
        },
    );
}
