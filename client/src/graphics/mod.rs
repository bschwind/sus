use bytemuck::{Pod, Zeroable};
use wgpu::{
    util::DeviceExt, BackendBit, BindGroup, Buffer, CommandEncoder, Device, Instance, Queue,
    RenderPipeline, Surface, SwapChain, SwapChainDescriptor, SwapChainTexture,
};
use winit::{dpi::PhysicalSize, window::Window};

pub mod text;

const CORNFLOWER_BLUE: wgpu::Color =
    wgpu::Color { r: 100.0 / 255.0, g: 149.0 / 255.0, b: 237.0 / 255.0, a: 1.0 };

pub struct GraphicsDevice {
    device: Device,
    queue: Queue,
    surface: Surface,
    swap_chain_descriptor: SwapChainDescriptor,
    swap_chain: SwapChain,
}

impl GraphicsDevice {
    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        // PRIMARY: All the apis that wgpu offers first tier of support for (Vulkan + Metal + DX12 + Browser WebGPU).
        let instance = Instance::new(BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let swapchain_format = wgpu::TextureFormat::Bgra8Unorm;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                // Prefer low power when on battery, high performance when on mains.
                power_preference: wgpu::PowerPreference::default(),
                // Request an adapter which can render to our surface
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropiate adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("GraphicsDevice device descriptor"),
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let swap_chain_descriptor = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox,
        };

        let swap_chain = device.create_swap_chain(&surface, &swap_chain_descriptor);

        Self { device, queue, surface, swap_chain_descriptor, swap_chain }
    }

    pub fn begin_frame(&mut self) -> FrameEncoder {
        let frame = self
            .swap_chain
            .get_current_frame()
            .expect("Failed to acquire next swap chain texture")
            .output;

        let encoder =
            self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("GraphicsDevice.begin_frame() encoder") });

        FrameEncoder { queue: &mut self.queue, frame, encoder }
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.swap_chain_descriptor.width = new_size.width;
        self.swap_chain_descriptor.height = new_size.height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.swap_chain_descriptor);
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn swap_chain_descriptor(&self) -> &SwapChainDescriptor {
        &self.swap_chain_descriptor
    }
}

pub struct FrameEncoder<'a> {
    queue: &'a mut Queue,
    pub frame: SwapChainTexture,
    pub encoder: CommandEncoder,
}

impl<'a> FrameEncoder<'a> {
    pub fn queue(&mut self) -> &mut Queue {
        &mut self.queue
    }

    // TODO(bschwind) - Maybe do this in a Drop impl
    pub fn finish(self) {
        self.queue.submit(Some(self.encoder.finish()));
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TexturedQuadVertex {
    pos: [f32; 2],
    uv: [f32; 2],
}

pub struct TexturedQuad {
    vertex_buf: Buffer,
    index_buf: Buffer,
    bind_group: BindGroup,
    pipeline: RenderPipeline,
}

impl TexturedQuad {
    pub fn new(graphics_device: &GraphicsDevice) -> Self {
        let vertex_data = vec![
            TexturedQuadVertex { pos: [-1.0, -1.0], uv: [0.0, 1.0] },
            TexturedQuadVertex { pos: [-1.0, 1.0], uv: [0.0, 0.0] },
            TexturedQuadVertex { pos: [1.0, 1.0], uv: [1.0, 0.0] },
            TexturedQuadVertex { pos: [1.0, -1.0], uv: [1.0, 1.0] },
        ];

        let index_data = vec![0u16, 1, 3, 2];

        let device = graphics_device.device();

        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertex_data),
            usage: wgpu::BufferUsage::VERTEX,
        });

        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&index_data),
            usage: wgpu::BufferUsage::INDEX,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("TexturedQuad bind group layout"),
            entries: &[],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("TexturedQuad pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("TexturedQuad bind group"),
            layout: &bind_group_layout,
            entries: &[],
        });

        let vertex_buffers = &[wgpu::VertexBufferLayout {
            array_stride: (std::mem::size_of::<TexturedQuadVertex>()) as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![
                0 => Float2, // pos
                1 => Float2, // uv
            ],
        }];

        let vs_module = device.create_shader_module(&wgpu::include_spirv!(
            "../../../resources/shaders/test.vert.spv"
        ));
        let fs_module = device.create_shader_module(&wgpu::include_spirv!(
            "../../../resources/shaders/test.frag.spv"
        ));

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("TexturedQuad render pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vs_module,
                entry_point: "main",
                buffers: vertex_buffers,
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: Some(wgpu::IndexFormat::Uint16),
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::Front,
                polygon_mode: wgpu::PolygonMode::Fill,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &fs_module,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: graphics_device.swap_chain_descriptor().format,
                    color_blend: wgpu::BlendState::REPLACE,
                    alpha_blend: wgpu::BlendState::REPLACE,
                    write_mask: wgpu::ColorWrite::ALL,
                }],
            }),
        });

        Self { vertex_buf, index_buf, pipeline, bind_group }
    }

    pub fn render(&self, frame_encoder: &mut FrameEncoder) {
        let frame = &frame_encoder.frame;
        let encoder = &mut frame_encoder.encoder;

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("TexturedQuad render pass"),
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment: &frame.view,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(CORNFLOWER_BLUE), store: true },
            }],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
        render_pass.draw_indexed(0..4 as u32, 0, 0..1);
    }
}
