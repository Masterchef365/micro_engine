use watertender::prelude::*;
use anyhow::Result;
use slotmap::{new_key_type, SecondaryMap};

new_key_type! {
    /// Handle for a Material (Draw commands)
    pub struct Shader;

    /// Handle for a Mesh (Draw content)
    pub struct Mesh;
}

// TODO: Make this expandable
const MAX_TRANSFORMS: usize = 2; 

pub struct DrawCmd {
    pub material: Shader,
    pub mesh: Mesh,
    pub transform: [f32; 4 * 4],
}

pub type FramePacket = Vec<DrawCmd>;

pub trait UserCode {
    fn init(&mut self, engine: &mut RenderEngine) -> Result<()>;
    fn frame(&mut self, engine: &mut RenderEngine) -> Result<FramePacket>;
    fn event(&mut self, engine: &mut RenderEngine, event: PlatformEvent) -> Result<()>;
}

/// RenderEngine mainloop integration, usercode execution
pub struct Main {
    engine: RenderEngine,
    user_code: Box<dyn UserCode>,
}

impl MainLoop for Main {
    type Args = Box<dyn UserCode>;
    fn new(&mut self, core: &SharedCore, mut platform: Platform<'_>, user_code: Self::Args) -> Result<Self> {
        let mut engine = RenderEngine::new(core, platform)?;
        user_code.init(&mut engine)?;
        Ok(Self {
            engine,
            user_code,
        })
    }

    fn frame(
        &mut self,
        frame: Frame,
        core: &SharedCore,
        platform: Platform<'_>,
    ) -> Result<PlatformReturn> {
        let packet = self.user_code.frame(&mut self.engine)?;
        self.engine.frame(frame, core, platform, packet)
    }

    fn swapchain_resize(&mut self, images: Vec<vk::Image>, extent: vk::Extent2D) -> Result<()> {
        self.starter_kit.swapchain_resize(images, extent)
    }

    fn event(
        &mut self,
        mut event: PlatformEvent<'_, '_>,
        _core: &Core,
        mut platform: Platform<'_>,
    ) -> Result<()> {
        self.user_code.event(&mut self.engine, event)?;
        self.engine.event(event, core, platform)
    }
}

impl SyncMainLoop for Main {
    fn winit_sync(&self) -> (vk::Semaphore, vk::Semaphore) {
        self.engine.winit_sync()
    }
}

pub struct RenderEngine {
    shaders: SecondaryMap<Shader, vk::Pipeline>,
    meshes: SecondaryMap<Mesh, ManagedMesh>,

    transforms: Vec<ManagedBuffer>,

    pipeline_layout: vk::PipelineLayout,
    scene_ubo: FrameDataUbo<SceneData>,
    camera: MultiPlatformCamera,
    anim: f32,
    starter_kit: StarterKit,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct SceneData {
    cameras: [f32; 4 * 4 * 2],
    anim: f32,
}

unsafe impl bytemuck::Zeroable for SceneData {}
unsafe impl bytemuck::Pod for SceneData {}

impl RenderEngine {
    /// Add a mesh, or replace an existing one with the same name
    pub fn add_mesh(&mut self, vertices: &[Vertex], indices: &[u32], key: Mesh) -> Result<()> {
        // Mesh uploads
        let mesh = upload_mesh(
            &mut self.starter_kit.staging_buffer,
            self.starter_kit.current_command_buffer(),
            vertices,
            indices,
        )?;
        self.meshes.insert(key, mesh);
        Ok(())
    }

    /// Add a shader, or replace an existing one with the same name
    pub fn add_shader(&mut self, vertex_spv: &[u8], fragment_spv: &[u8], topo: vk::PrimitiveTopology, key: Shader) -> Result<()> {
        let pipeline = shader(
            &self.starter_kit.core,
            vertex_spv,
            fragment_spv,
            topo,
            self.starter_kit.render_pass,
            self.pipeline_layout,
        )?;
        self.shaders.insert(key, pipeline);
        Ok(())
    }
}

impl RenderEngine {
    fn new(core: &SharedCore, mut platform: Platform<'_>) -> Result<Self> {
        let mut starter_kit = StarterKit::new(core.clone(), &mut platform)?;

        // Camera
        let camera = MultiPlatformCamera::new(&mut platform);

        // Scene data
        let scene_ubo = FrameDataUbo::new(core.clone(), defaults::FRAMES_IN_FLIGHT)?;

        let descriptor_set_layouts = [scene_ubo.descriptor_set_layout()];

        // Pipeline layout
        let push_constant_ranges = [vk::PushConstantRangeBuilder::new()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(std::mem::size_of::<[f32; 4 * 4]>() as u32)];

        let create_info = vk::PipelineLayoutCreateInfoBuilder::new()
            .push_constant_ranges(&push_constant_ranges)
            .set_layouts(&descriptor_set_layouts);

        let pipeline_layout =
            unsafe { core.device.create_pipeline_layout(&create_info, None, None) }.result()?;


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
            let frame_data_bi = [scene_data.descriptor_buffer_info(frame)];
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
            anim: 0.0,
            pipeline_layout,
            scene_ubo,
            starter_kit,
            user_code: None,
            meshes: SecondaryMap::new(),
            shaders: SecondaryMap::new(),
        };

        Ok(instance)
    }

    fn frame(
        &mut self,
        frame: Frame,
        core: &SharedCore,
        platform: Platform<'_>,
        packet: FramePacket,
    ) -> Result<PlatformReturn> {
        let cmd = self.starter_kit.begin_command_buffer(frame)?;
        let command_buffer = cmd.command_buffer;

        unsafe {
            core.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &[self.scene_ubo.descriptor_set(self.starter_kit.frame)],
                &[],
            );

            // Draw cmds
            core.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );

            draw_meshes(
                core,
                command_buffer,
                std::slice::from_ref(&&self.rainbow_cube),
            );
        }

        let (ret, cameras) = self.camera.get_matrices(platform)?;

        self.scene_ubo.upload(
            self.starter_kit.frame,
            &SceneData {
                cameras,
                anim: self.anim,
            },
        )?;

        // End draw cmds
        self.starter_kit.end_command_buffer(cmd)?;

        Ok(ret)
    }

    fn swapchain_resize(&mut self, images: Vec<vk::Image>, extent: vk::Extent2D) -> Result<()> {
        self.starter_kit.swapchain_resize(images, extent)
    }

    fn event(
        &mut self,
        mut event: PlatformEvent<'_, '_>,
        _core: &Core,
        mut platform: Platform<'_>,
    ) -> Result<()> {
        self.camera.handle_event(&mut event, &mut platform);
        starter_kit::close_when_asked(event, platform);
        Ok(())
    }
}

impl SyncMainLoop for RenderEngine {
    fn winit_sync(&self) -> (vk::Semaphore, vk::Semaphore) {
        self.starter_kit.winit_sync()
    }
}