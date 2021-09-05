use anyhow::Result;
use defaults::FRAMES_IN_FLIGHT;
use slotmap::{new_key_type, SecondaryMap};
use watertender::memory;
use watertender::prelude::*;

// TODO: Make this expandable
const MAX_TRANSFORMS: usize = 5000;

new_key_type! {
    /// Handle for a Material (Draw commands)
    pub struct Shader;

    /// Handle for a Mesh (Draw content)
    pub struct Mesh;
}

impl mlua::UserData for Shader {}
impl mlua::UserData for Mesh {}

/// Transform data in column-major format
pub type Transform = [[f32; 4]; 4];
pub const TRANSFORM_IDENTITY: Transform = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// A single object to be drawn
pub struct DrawCmd {
    pub shader: Shader,
    pub geometry: DrawGeometry,
    pub transform: Option<Transform>,
}

#[derive(Debug, Clone, Copy)]
pub enum DrawGeometry {
    /// Draw a mesh from a buffer
    Mesh {
        /// Mesh to be drawn
        mesh: Mesh,
        /// Maximum index; if None the entire mesh will be drawn
        max_idx: Option<u32>,
    },
    /// Draw a mesh without a buffer
    Procedural {
        n_verts: u32,
    }
}

/// A set of draw commands
pub struct FramePacket {
    pub cmds: Vec<DrawCmd>,
    pub midi: [u32; 3],
    pub anim: f32,
}

/// Rendering engine
pub struct RenderEngine {
    shaders: SecondaryMap<Shader, vk::Pipeline>,
    meshes: SecondaryMap<Mesh, ManagedMesh>,

    transforms: Vec<ManagedBuffer>,

    descriptor_sets: Vec<vk::DescriptorSet>,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set_layout: vk::DescriptorSetLayout,

    pipeline_layout: vk::PipelineLayout,
    scene_ubo: FrameDataUbo<SceneData>,
    camera: MultiPlatformCamera,
    starter_kit: StarterKit,
}

/// Scene data passed to shaders
#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct SceneData {
    cameras: [f32; 4 * 4 * 2],
    anim: f32,
    midi: [u32; 3],
}

unsafe impl bytemuck::Zeroable for SceneData {}
unsafe impl bytemuck::Pod for SceneData {}

impl RenderEngine {
    /// Add a mesh, or replace an existing one with the same name
    pub fn add_mesh(&mut self, vertices: &[Vertex], indices: &[u32], key: Mesh) -> Result<()> {
        // Mesh uploads
        let cmd = self.starter_kit.current_command_buffer();
        let mesh = upload_mesh(&mut self.starter_kit.staging_buffer, cmd, vertices, indices)?;
        self.meshes.insert(key, mesh);
        Ok(())
    }

    /// Add a shader, or replace an existing one with the same name
    pub fn add_shader(
        &mut self,
        vertex_spv: &[u8],
        fragment_spv: &[u8],
        topo: vk::PrimitiveTopology,
        key: Shader,
    ) -> Result<()> {
        let pipeline = shader(
            &self.starter_kit.core,
            vertex_spv,
            fragment_spv,
            topo,
            self.starter_kit.render_pass,
            self.pipeline_layout,
        )?;
        let maybe_old_pipeline = self.shaders.insert(key, pipeline);
        unsafe {
            let core = &self.starter_kit.core;
            if maybe_old_pipeline.is_some() {
                core.device.queue_wait_idle(core.queue).result()?;
                core.device.destroy_pipeline(maybe_old_pipeline, None);
            }
        }
        Ok(())
    }
}

impl RenderEngine {
    /// Initialize the engine
    pub fn new(core: &SharedCore, mut platform: Platform<'_>) -> Result<Self> {
        let starter_kit = StarterKit::new(core.clone(), &mut platform)?;

        // Camera
        let camera = MultiPlatformCamera::new(&mut platform);

        // Scene data
        let scene_ubo = FrameDataUbo::new(core.clone(), defaults::FRAMES_IN_FLIGHT)?;

        // Transforms data
        let total_size = std::mem::size_of::<Transform>() * MAX_TRANSFORMS;
        let ci = vk::BufferCreateInfoBuilder::new()
            .size(total_size as u64)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .usage(vk::BufferUsageFlags::STORAGE_BUFFER);
        let transforms = (0..FRAMES_IN_FLIGHT)
            .map(|_| ManagedBuffer::new(core.clone(), ci, memory::UsageFlags::UPLOAD))
            .collect::<Result<Vec<_>>>()?;

        // Create descriptor set layout
        const FRAME_DATA_BINDING: u32 = 0;
        const TRANSFORM_BINDING: u32 = 1;
        let bindings = [
            vk::DescriptorSetLayoutBindingBuilder::new()
                .binding(FRAME_DATA_BINDING)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::ALL_GRAPHICS),
            vk::DescriptorSetLayoutBindingBuilder::new()
                .binding(TRANSFORM_BINDING)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::ALL_GRAPHICS),
        ];

        let descriptor_set_layout_ci =
            vk::DescriptorSetLayoutCreateInfoBuilder::new().bindings(&bindings);

        let descriptor_set_layout = unsafe {
            core.device
                .create_descriptor_set_layout(&descriptor_set_layout_ci, None, None)
        }
        .result()?;

        // Create descriptor pool
        let pool_sizes = [
            vk::DescriptorPoolSizeBuilder::new()
                ._type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(FRAMES_IN_FLIGHT as _),
            vk::DescriptorPoolSizeBuilder::new()
                ._type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(FRAMES_IN_FLIGHT as _),
        ];

        let create_info = vk::DescriptorPoolCreateInfoBuilder::new()
            .pool_sizes(&pool_sizes)
            .max_sets((FRAMES_IN_FLIGHT * 2) as _);

        let descriptor_pool =
            unsafe { core.device.create_descriptor_pool(&create_info, None, None) }.result()?;

        // Create descriptor sets
        let layouts = vec![descriptor_set_layout; FRAMES_IN_FLIGHT];
        let create_info = vk::DescriptorSetAllocateInfoBuilder::new()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&layouts);

        let descriptor_sets =
            unsafe { core.device.allocate_descriptor_sets(&create_info) }.result()?;

        // Write descriptor sets
        for (frame, &descriptor_set) in descriptor_sets.iter().enumerate() {
            let frame_data_bi = [scene_ubo.descriptor_buffer_info(frame)];
            let transform_bi = [vk::DescriptorBufferInfoBuilder::new()
                .buffer(transforms[frame].instance())
                .offset(0)
                .range(vk::WHOLE_SIZE)];

            let writes = [
                vk::WriteDescriptorSetBuilder::new()
                    .buffer_info(&frame_data_bi)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .dst_set(descriptor_set)
                    .dst_binding(FRAME_DATA_BINDING)
                    .dst_array_element(0),
                vk::WriteDescriptorSetBuilder::new()
                    .buffer_info(&transform_bi)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .dst_set(descriptor_set)
                    .dst_binding(TRANSFORM_BINDING)
                    .dst_array_element(0),
            ];

            unsafe {
                core.device.update_descriptor_sets(&writes, &[]);
            }
        }

        // Pipeline layout
        let push_constant_ranges = [vk::PushConstantRangeBuilder::new()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(std::mem::size_of::<[f32; 4 * 4]>() as u32)];

        let descriptor_set_layouts = [descriptor_set_layout];
        let create_info = vk::PipelineLayoutCreateInfoBuilder::new()
            .push_constant_ranges(&push_constant_ranges)
            .set_layouts(&descriptor_set_layouts);

        let pipeline_layout =
            unsafe { core.device.create_pipeline_layout(&create_info, None, None) }.result()?;

        let instance = Self {
            camera,
            pipeline_layout,
            scene_ubo,
            starter_kit,
            transforms,
            descriptor_set_layout,
            descriptor_sets,
            descriptor_pool,
            meshes: SecondaryMap::new(),
            shaders: SecondaryMap::new(),
        };

        Ok(instance)
    }

    pub fn frame(
        &mut self,
        frame: Frame,
        core: &SharedCore,
        platform: Platform<'_>,
        packet: FramePacket,
    ) -> Result<PlatformReturn> {
        // Collect and write transforms
        let mut positions: Vec<Transform> = vec![TRANSFORM_IDENTITY];
        let mut cmd_transform_indices: Vec<u32> = Vec::with_capacity(packet.cmds.len());
        for cmd in &packet.cmds {
            if let Some(transform) = cmd.transform {
                let idx = positions.len().min(MAX_TRANSFORMS) as u32;
                cmd_transform_indices.push(idx);
                positions.push(transform);
            } else {
                cmd_transform_indices.push(0);
            }
        }
        assert_eq!(packet.cmds.len(), cmd_transform_indices.len());

        if positions.len() > MAX_TRANSFORMS {
            eprintln!("Too many positions! {} exceeded {}", positions.len(), MAX_TRANSFORMS);
            positions.truncate(MAX_TRANSFORMS);
        }

        self.transforms[self.starter_kit.frame]
            .write_bytes(0, bytemuck::cast_slice(positions.as_slice()))?;

        let command_buffer_start = self.starter_kit.begin_command_buffer(frame)?;

        // Write command buffer
        let command_buffer = command_buffer_start.command_buffer;
        unsafe {
            core.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &[self.descriptor_sets[self.starter_kit.frame]],
                &[],
            );

            // Draw cmds
            // TODO: Batch draw calls per pipeline...
            for (cmd, transf_idx) in packet.cmds.into_iter().zip(cmd_transform_indices) {

                let shader = match self.shaders.get(cmd.shader) {
                    Some(s) => s,
                    None => {
                        //eprintln!("Shader unavailable!");
                        continue;
                    }
                };

                core.device.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    *shader,
                );

                let push_const = [transf_idx];
                // TODO: Make this a shortcut
                core.device.cmd_push_constants(
                    command_buffer,
                    self.pipeline_layout,
                    vk::ShaderStageFlags::VERTEX,
                    0,
                    std::mem::size_of_val(&push_const) as u32,
                    push_const.as_ptr() as _,
                );

                match cmd.geometry {
                    DrawGeometry::Mesh { mesh, max_idx } => {
                        let mesh = match self.meshes.get(mesh) {
                            Some(m) => m,
                            None => {
                                //eprintln!("Mesh unavailable!");
                                continue;
                            }
                        };

                        core.device.cmd_bind_vertex_buffers(
                            command_buffer,
                            0,
                            &[mesh.vertices.instance()],
                            &[0],
                        );

                        core.device.cmd_bind_index_buffer(
                            command_buffer,
                            mesh.indices.instance(),
                            0,
                            vk::IndexType::UINT32,
                        );

                        let max_idx = match max_idx {
                            Some(max) => max.min(mesh.n_indices),
                            None => mesh.n_indices,
                        };

                        core.device.cmd_draw_indexed(command_buffer, max_idx, 1, 0, 0, 0);
                    },
                    DrawGeometry::Procedural { n_verts } => {
                        core.device.cmd_draw(command_buffer, n_verts, 1, 0, 0);
                    },
                }
            }
        }

        let (ret, cameras) = self.camera.get_matrices(&platform)?;

        self.scene_ubo.upload(
            self.starter_kit.frame,
            &SceneData {
                cameras,
                anim: packet.anim,
                midi: packet.midi,
            },
        )?;

        // End draw cmds
        self.starter_kit.end_command_buffer(command_buffer_start)?;

        Ok(ret)
    }

    pub fn swapchain_resize(&mut self, images: Vec<vk::Image>, extent: vk::Extent2D) -> Result<()> {
        self.starter_kit.swapchain_resize(images, extent)
    }

    pub fn event(
        &mut self,
        mut event: PlatformEvent<'_, '_>,
        _core: &Core,
        mut platform: Platform<'_>,
    ) -> Result<()> {
        self.camera.handle_event(&mut event, &mut platform);
        starter_kit::close_when_asked(event, platform);
        Ok(())
    }

    pub fn winit_sync(&self) -> (vk::Semaphore, vk::Semaphore) {
        self.starter_kit.winit_sync()
    }
}

impl Drop for RenderEngine {
    fn drop(&mut self) {
        unsafe {
            self.starter_kit.core.device.destroy_descriptor_pool(Some(self.descriptor_pool), None);
            self.starter_kit.core.device.destroy_descriptor_set_layout(Some(self.descriptor_set_layout), None);
            self.starter_kit.core.device.destroy_pipeline_layout(Some(self.pipeline_layout), None);
            for (_, pipeline) in self.shaders.drain() {
                self.starter_kit.core.device.destroy_pipeline(Some(pipeline), None);
            }
        }
    }
}

impl Default for FramePacket {
    fn default() -> Self {
        FramePacket {
            cmds: vec![],
            anim: 0.,
            midi: [0; 3],
        }
    }
}
