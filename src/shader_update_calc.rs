use crate::engine::{Shader, RenderEngine};
use slotmap::SecondaryMap;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use watertender::vk::PrimitiveTopology;
use anyhow::Result;
use shaderc::{ShaderKind, CompilationArtifact, Compiler};

#[derive(Clone, Debug, Hash, PartialEq)]
pub struct UniquePipeline {
    pub vertex_path: String,
    pub fragment_path: String,
    pub primitive: PrimitiveTopology,
}

pub struct ShaderUpdateCalculator {
    shader_to_unique: SecondaryMap<Shader, UniquePipeline>,
    path_to_shader: HashMap<PathBuf, Vec<Shader>>,
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

    pub fn track_shader(&mut self, handle: Shader, unique: UniquePipeline) -> Result<()> {
        self.shader_to_unique.insert(handle, unique.clone());

        self.path_to_shader
            .entry(std::fs::canonicalize(unique.vertex_path)?)
            .or_default()
            .push(handle);

        self.path_to_shader
            .entry(std::fs::canonicalize(unique.fragment_path)?)
            .or_default()
            .push(handle);

        self.updates.insert(handle);

        Ok(())
    }

    pub fn shader_file_touched(&mut self, path: &std::path::Path) {
        if let Some(path) = path.canonicalize().ok() {
            let maybe_shaders = self.path_to_shader.get(&path);
            if let Some(shaders) = maybe_shaders {
                self.updates.extend(shaders);
            }
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

pub fn compile_jobs(compiler: &mut Compiler, jobs: &[(Shader, UniquePipeline)], engine: &mut RenderEngine) -> Result<()> {
    // Cache to speed up compilation of shaders. Probably unneeded but I wanted to okay
    let mut fragment_artefacts: HashMap<String, Option<CompilationArtifact>> = HashMap::new();
    let mut vertex_artefacts: HashMap<String, Option<CompilationArtifact>> = HashMap::new();

    for (shader, paths) in jobs {
        let vertex = vertex_artefacts
            .entry(paths.vertex_path.clone())
            .or_insert_with(|| compile_nice(compiler, &paths.vertex_path, ShaderKind::Vertex));
        let fragment = fragment_artefacts
            .entry(paths.fragment_path.clone())
            .or_insert_with(|| compile_nice(compiler, &paths.fragment_path, ShaderKind::Fragment));
        if let Some((vertex, fragment)) = vertex.as_ref().zip(fragment.as_ref()) {
            engine.add_shader(&vertex.as_binary_u8(), fragment.as_binary_u8(), paths.primitive, *shader)?;
        }
    }

    Ok(())
}

fn compile_nice(compiler: &mut Compiler, path: &str, kind: ShaderKind) -> Option<CompilationArtifact> {
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to load shader source at path \"{}\"; {}", path, e);
            return None;
        }
    };

    let res = compiler.compile_into_spirv(&src, kind, path, "main", None);
    match res {
        Ok(art) => Some(art),
        Err(e) => {
            eprintln!("Failed to compile shader at path \"{}\"; {}", path, e);
            None
        }
    }
}
