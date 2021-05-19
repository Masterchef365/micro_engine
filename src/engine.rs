use watertender::prelude::*;
use anyhow::Result;
use slotmap::{new_key_type, SecondaryMap};

new_key_type! {
    /// Handle for a Material (Draw commands)
    pub struct Shader;

    /// Handle for a Mesh (Draw content)
    pub struct Mesh;
}

pub struct DrawCmd {
    pub material: Shader,
    pub mesh: Mesh,
    pub transform: [f32; 4 * 4],
}

pub type FramePacket = Vec<DrawCmd>;

pub trait UserCode {
    fn init(&mut self, engine: &mut RenderEngine);
    fn frame(&mut self, engine: &mut RenderEngine) -> FramePacket;
    fn event(&mut self, engine: &mut RenderEngine, event: PlatformEvent);
}

pub struct RenderEngine {
    shaders: SecondaryMap<Shader, vk::Pipeline>,
    meshes: SecondaryMap<Mesh, ManagedMesh>,

    pipeline_layout: vk::PipelineLayout,
    scene_ubo: FrameDataUbo<SceneData>,
    camera: MultiPlatformCamera,
    anim: f32,
    starter_kit: StarterKit,
    user_code: Box<dyn UserCode>,
    core: SharedCore,
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
    pub fn add_mesh(&mut self, vertices: &[Vertex], indices: &[u16], key: Mesh) {
        // Mesh uploads
        let (vertices, indices) = rainbow_cube();
        let rainbow_cube = upload_mesh(
            &mut starter_kit.staging_buffer,
            starter_kit.command_buffers[0],
            &vertices,
            &indices,
        )?;
    }

    /// Add a shader, or replace an existing one with the same name
    pub fn add_shader(&mut self, vertex_spv: &[u8], fragment_spv: &[u8], topo: vk::PrimitiveTopology, key: Shader) {
        let pipeline = shader(
            self.core.clone(),
            vertex_spv,
            fragment_spv,
            topo,
            starter_kit.render_pass,
            pipeline_layout,
        )?;
    }
}

impl MainLoop for RenderEngine {
    type Args = Box<dyn UserCode>;
    fn new(core: &SharedCore, mut platform: Platform<'_>, user: Self::Args) -> Result<Self> {
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

        Ok(Self {
            camera,
            anim: 0.0,
            pipeline_layout,
            scene_ubo,
            starter_kit,
            core: core.clone(),
            meshes: SecondaryMap::new(),
            shaders: SecondaryMap::new(),
        })
    }

    fn frame(
        &mut self,
        frame: Frame,
        core: &SharedCore,
        platform: Platform<'_>,
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

fn rainbow_cube() -> (Vec<Vertex>, Vec<u32>) {
    let vertices = vec![
        Vertex::new([-1.0, -1.0, -1.0], [0.0, 1.0, 1.0]),
        Vertex::new([1.0, -1.0, -1.0], [1.0, 0.0, 1.0]),
        Vertex::new([1.0, 1.0, -1.0], [1.0, 1.0, 0.0]),
        Vertex::new([-1.0, 1.0, -1.0], [0.0, 1.0, 1.0]),
        Vertex::new([-1.0, -1.0, 1.0], [1.0, 0.0, 1.0]),
        Vertex::new([1.0, -1.0, 1.0], [1.0, 1.0, 0.0]),
        Vertex::new([1.0, 1.0, 1.0], [0.0, 1.0, 1.0]),
        Vertex::new([-1.0, 1.0, 1.0], [1.0, 0.0, 1.0]),
    ];

    let indices = vec![
        3, 1, 0, 2, 1, 3, 2, 5, 1, 6, 5, 2, 6, 4, 5, 7, 4, 6, 7, 0, 4, 3, 0, 7, 7, 2, 3, 6, 2, 7,
        0, 5, 4, 1, 5, 0,
    ];

    (vertices, indices)
}
