use crate::engine::Shader;
use slotmap::SecondaryMap;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use watertender::vk::PrimitiveTopology;

#[derive(Clone, Debug, Hash, PartialEq)]
pub struct UniquePipeline {
    pub vertex_path: String,
    pub fragment_path: String,
    pub primitive: PrimitiveTopology,
}

pub struct ShaderUpdateCalculator {
    shader_to_unique: SecondaryMap<Shader, UniquePipeline>,
    path_to_shader: HashMap<String, Vec<Shader>>,
    updates: HashSet<Shader>,
}

impl ShaderUpdateCalculator {
    pub fn new() -> Self {
        Self {
            shader_to_unique: Default::default(),
            path_to_shader: Default::default(),
            updates: Default::default(),
        }
    }

    pub fn track_shader(&mut self, handle: Shader, unique: UniquePipeline) {
        self.shader_to_unique.insert(handle, unique.clone());

        self.path_to_shader
            .entry(unique.vertex_path)
            .or_default()
            .push(handle);

        self.path_to_shader
            .entry(unique.fragment_path)
            .or_default()
            .push(handle);

        self.updates.insert(handle);
    }

    pub fn shader_file_touched(&mut self, path: &str) {
        if let Some(shaders) = self.path_to_shader.get(path) {
            self.updates.extend(shaders);
        }
    }

    pub fn updates(&mut self) -> Vec<(Shader, UniquePipeline)> {
        let updates = self.updates
            .iter()
            .map(|handle| (
                *handle,
                self.shader_to_unique.get(*handle).unwrap().clone(),
            ))
            .collect();
        self.updates.clear();
        updates
    }
}
