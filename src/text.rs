use crate::graphics::{FrameEncoder, GraphicsDevice};
use fontdue::{
    layout::{GlyphPosition, HorizontalAlign, Layout, LayoutSettings, TextStyle, VerticalAlign},
    Font as FontdueFont, FontSettings, Metrics,
};
use gpu::GlyphPainter;
use rect_packer::Packer;
use std::collections::{hash_map::Entry, HashMap};
use wgpu::Texture;
use winit::dpi::PhysicalSize;

const BITMAP_WIDTH: u32 = 4096;
const BITMAP_HEIGHT: u32 = 4096;
const BORDER_PADDING: u32 = 2;
const RECTANGLE_PADDING: u32 = 2;

pub const WHITE: Color = Color::new(255, 255, 255, 255);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Font {
    SpaceMono400(u32),
}

impl Font {
    fn size(&self) -> u32 {
        use Font::*;

        match self {
            SpaceMono400(size) => *size,
        }
    }

    fn font_bytes(&self) -> &'static [u8] {
        match self {
            Font::SpaceMono400(_) => include_bytes!("../resources/fonts/space_mono_400.ttf"),
        }
    }
}

struct FontData {
    /// A map of Fonts to their indices in `rasterizers` and `fonts`.
    rasterizer_indices: HashMap<Font, usize>,
    rasterizers: Vec<FontdueFont>,
    fonts: Vec<Font>,
}

impl FontData {
    fn new() -> Self {
        FontData { rasterizer_indices: HashMap::new(), rasterizers: Vec::new(), fonts: Vec::new() }
    }

    /// Creates and stores a rasterizer for this Font if one doesn't already exist.
    fn create_rasterizer(&mut self, font: Font) {
        // Asserting this as it otherwise causes a sudden segfault.
        assert!(font.size() > 0, "expecting a positive font size");

        if let Entry::Vacant(entry) = self.rasterizer_indices.entry(font) {
            let font_index = self.rasterizers.len();

            let rasterizer = FontdueFont::from_bytes(
                font.font_bytes(),
                FontSettings { scale: font.size() as f32, ..FontSettings::default() },
            )
            .unwrap();

            self.rasterizers.push(rasterizer);
            self.fonts.push(font);
            entry.insert(font_index);
        }
    }

    fn rasterizer_for_font(&self, font: &Font) -> Option<&FontdueFont> {
        self.rasterizer_indices.get(&font).map(|font_index| &self.rasterizers[*font_index])
    }

    fn font_index(&self, font: &Font) -> Option<usize> {
        self.rasterizer_indices.get(font).copied()
    }

    fn font(&self, font_index: usize) -> Option<&Font> {
        self.fonts.get(font_index)
    }

    fn rasterizers(&self) -> &[FontdueFont] {
        &self.rasterizers
    }
}

#[derive(Debug)]
enum RasterizeResult {
    /// The glyph exists and was successfully packed into the
    /// the glyph texture.
    Packed,

    /// The glyph was a whitespace character which doesn't need
    /// to be packed into the glyph texture.
    WhitespaceChar,

    // Issue here: https://github.com/mooman219/fontdue/issues/43
    /// The glyph was missing, but a fallback character was still
    /// packed into the glyph texture.
    GlyphMissing,
}

#[derive(Debug)]
pub enum RasterizationError {
    NoTextureSpace,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StyledCharacter {
    pub character: char,
    pub font: Font,
}

#[derive(Debug, Clone)]
pub struct CharacterMetadata {
    metrics: Metrics,
    texture_x: f32,      // Texture space
    texture_y: f32,      // Texture space
    texture_width: f32,  // Texture space
    texture_height: f32, // Texture space
}

pub struct StyledText<'a> {
    pub text: &'a str,
    pub font: Font,
    pub color: Color,
}

impl<'a> StyledText<'a> {
    pub fn default_styling(text: &'a str) -> Self {
        StyledText { text, font: Font::SpaceMono400(24), color: WHITE }
    }
}

/// Where to align on a particular axis.
/// Y: Start = top of the text box aligned to the Y coord
///    End   = bottom of the text box aligned to the Y coord
/// X: Start = left side of the text box aligned to the X coord
///    End   = right side of the text box aligned to the X coord
/// Units are in pixels.
#[derive(Debug)]
pub enum AxisAlign {
    Start(i32),
    End(i32),
    Center(i32),
    WindowCenter,
}

impl Default for AxisAlign {
    fn default() -> Self {
        AxisAlign::Start(0)
    }
}

/// Describes alignment for a block of text. Max width
/// and height are optional and default to the window width
/// and height.
#[derive(Debug, Default)]
pub struct TextAlignment {
    pub x: AxisAlign,
    pub y: AxisAlign,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
}

impl TextAlignment {
    pub fn new(x: AxisAlign, y: AxisAlign) -> Self {
        Self { x, y, max_width: None, max_height: None }
    }

    pub fn left_top(x: i32, y: i32) -> Self {
        Self { x: AxisAlign::Start(x), y: AxisAlign::Start(y), max_width: None, max_height: None }
    }

    fn into_layout_settings(self, window_size: PhysicalSize<u32>) -> LayoutSettings {
        let window_width = window_size.width as i32;
        let window_height = window_size.height as i32;
        let max_width = self.max_width.unwrap_or(window_width as u32) as i32;
        let max_height = self.max_height.unwrap_or(window_height as u32) as i32;

        let (x, horizontal_align) = match self.x {
            AxisAlign::Start(x) => (x, HorizontalAlign::Left),
            AxisAlign::End(x) => (x - max_width, HorizontalAlign::Right),
            AxisAlign::Center(x) => (x - (max_width / 2), HorizontalAlign::Center),
            AxisAlign::WindowCenter => {
                ((window_width / 2) - (max_width / 2), HorizontalAlign::Center)
            },
        };

        let (y, vertical_align) = match self.y {
            AxisAlign::Start(y) => (y, VerticalAlign::Top),
            AxisAlign::End(y) => (y - max_height, VerticalAlign::Bottom),
            AxisAlign::Center(y) => (y - (max_height / 2), VerticalAlign::Middle),
            AxisAlign::WindowCenter => {
                ((window_height / 2) - (max_height / 2), VerticalAlign::Middle)
            },
        };

        LayoutSettings {
            x: x as f32,
            y: -y as f32,
            max_width: Some(max_width as f32),
            max_height: Some(max_height as f32),
            include_whitespace: true,
            horizontal_align,
            vertical_align,
            ..LayoutSettings::default()
        }
    }
}

pub struct TextSystem {
    font_data: FontData,

    /// A map of styled characters to their associated metadata
    /// (their location in the font bitmap, width, height, etc.)
    char_metadata: HashMap<StyledCharacter, CharacterMetadata>,

    /// Data structure to pack glyph rectangles into a larger GPU bitmap.
    glyph_packer: Packer,

    /// Object to perform text layout on content blocks.
    layout: Layout,

    /// GPU-side texture.
    glyph_texture: Texture,

    /// GPU glyph renderer.
    glpyh_painter: GlyphPainter,
}

impl TextSystem {
    pub fn new(graphics_device: &GraphicsDevice) -> Self {
        let font_data = FontData::new();
        let char_metadata = HashMap::new();

        let packer_config = rect_packer::Config {
            width: BITMAP_WIDTH as i32,
            height: BITMAP_HEIGHT as i32,
            border_padding: BORDER_PADDING as i32,
            rectangle_padding: RECTANGLE_PADDING as i32,
        };

        let glyph_packer = Packer::new(packer_config);
        let layout = Layout::new();

        let glyph_texture_extent =
            wgpu::Extent3d { width: BITMAP_WIDTH, height: BITMAP_HEIGHT, depth: 1 };

        let device = graphics_device.device();

        let glyph_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Glyph texture"),
            size: glyph_texture_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
        });

        let glpyh_painter = GlyphPainter::new(graphics_device);

        Self { font_data, char_metadata, glyph_packer, layout, glyph_texture, glpyh_painter }
    }

    pub fn render(&self, frame_encoder: &mut FrameEncoder) {}

    /// Rasterizes and caches this character in the glyph texture.
    /// Returns Some(RasterizeResult) if the character is packed into the texture,
    /// otherwise None.
    fn rasterize_and_cache(
        &mut self,
        c: StyledCharacter,
        frame_encoder: &mut FrameEncoder,
    ) -> Result<RasterizeResult, RasterizationError> {
        let metadata = self.char_metadata.entry(c);

        match metadata {
            Entry::Occupied(_) => {
                // Good to go, this character already exists
                Ok(RasterizeResult::Packed)
            },
            Entry::Vacant(entry) => {
                let styled_char = entry.key();

                let character = styled_char.character;
                let font_size = styled_char.font.size() as f32;

                let rasterizer =
                    self.font_data.rasterizer_for_font(&styled_char.font).unwrap_or_else(|| {
                        panic!("Rasterizer should exist for Font: {:?}", styled_char.font)
                    });

                let (metrics, bitmap) = rasterizer.rasterize(character, font_size);
                let can_rotate = false;

                if metrics.width == 0 || metrics.height == 0 {
                    // This was likely a whitespace character which isn't missing from the font
                    // but does not have an actual bitmap. The rectangle packer would fail on
                    // this case so we return here as everything will still work.
                    return Ok(RasterizeResult::WhitespaceChar);
                }

                if let Some(packed_rect) =
                    self.glyph_packer.pack(metrics.width as i32, metrics.height as i32, can_rotate)
                {
                    let float_width = BITMAP_WIDTH as f32;
                    let float_height = BITMAP_HEIGHT as f32;

                    let char_metadata = CharacterMetadata {
                        metrics,
                        texture_x: packed_rect.x as f32 / float_width,
                        texture_y: packed_rect.y as f32 / float_height,
                        texture_width: packed_rect.width as f32 / float_width,
                        texture_height: packed_rect.height as f32 / float_height,
                    };

                    entry.insert(char_metadata);

                    Self::write_to_texture(
                        frame_encoder,
                        &self.glyph_texture,
                        &bitmap,
                        packed_rect.x as u32,
                        packed_rect.y as u32,
                        packed_rect.width as u32,
                        packed_rect.height as u32,
                    );

                    let glyph_missing = rasterizer.lookup_glyph_index(character) == 0;

                    if glyph_missing {
                        Ok(RasterizeResult::GlyphMissing)
                    } else {
                        Ok(RasterizeResult::Packed)
                    }
                } else {
                    // Couldn't pack into texture, resize it
                    // warn!("Couldn't pack char: {:?} into glyph texture", character);
                    Err(RasterizationError::NoTextureSpace)
                }
            },
        }
    }

    fn write_to_texture(
        frame_encoder: &mut FrameEncoder,
        texture: &Texture,
        bitmap: &[u8],
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) {
        let bitmap_texture_extent = wgpu::Extent3d { width, height, depth: 1 };

        frame_encoder.queue().write_texture(
            wgpu::TextureCopyView { texture, mip_level: 0, origin: wgpu::Origin3d { x, y, z: 0 } },
            bitmap,
            wgpu::TextureDataLayout { offset: 0, bytes_per_row: 4 * width, rows_per_image: 0 },
            bitmap_texture_extent,
        );
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl Color {
    pub const fn new(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self { red, green, blue, alpha }
    }
}

mod gpu {
    use crate::GraphicsDevice;
    use bytemuck::{Pod, Zeroable};
    use wgpu::{
        util::DeviceExt, BackendBit, BindGroup, Buffer, BufferDescriptor, CommandEncoder, Device,
        Instance, Queue, RenderPipeline, Surface, SwapChain, SwapChainDescriptor, SwapChainTexture,
    };

    const MAX_INSTANCE_COUNT: usize = 40_000;

    /// Vertex attributes for instanced glyph data.
    #[derive(Debug, Copy, Clone)]
    struct GlyphInstanceData {
        /// XY position of the bottom left of the glyph in pixels
        pos: [f32; 2],

        /// The width and height of the rendered glyph, in pixels.
        size: [f32; 2],

        /// The UV coordinates of the top-left corner of the glpyh
        /// and the width/height of the glyph, both in texture space.
        uv_extents: [f32; 4],

        /// The color of the glyph, including alpha.
        color: [f32; 4],
    }

    impl Default for GlyphInstanceData {
        fn default() -> Self {
            GlyphInstanceData {
                pos: [0.0, 0.0],
                size: [0.0, 0.0],
                uv_extents: [0.0, 0.0, 0.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            }
        }
    }

    /// Vertex attributes for our single glpyh quad.
    #[repr(C)]
    #[derive(Debug, Copy, Clone, Pod, Zeroable)]
    struct GlyphQuadVertex {
        /// UV coordinates for one vertex, in texture space.
        uv: [f32; 2],
    }

    /// This font renderer uses instanced rendering to draw quads for each
    /// glyph.
    /// Reference: https://learnopengl.com/Advanced-OpenGL/Instancing
    /// A single "unit quad" is stored in the vertex buffer. It only requires
    /// the default UV data for each vertex (0.0 - 1.0).
    /// There is also a dynamic vertex buffer. Each element in this buffer stores
    /// the data required to render one glyph. We update this buffer when the font
    /// system tells us where and how many glyphs to render.
    pub struct GlyphPainter {
        // glyph_vertex_buffer: glium::VertexBuffer<GlyphQuadVertex>,
        // index_buffer: glium::IndexBuffer<u16>,
        // instance_buffer: glium::VertexBuffer<GlyphInstanceData>,
        // shader: glium::Program,
        glyph_vertex_buffer: Buffer,
        index_buffer: Buffer,
        instance_buffer: Buffer,
        bind_group: BindGroup,
        pipeline: RenderPipeline,
    }

    impl GlyphPainter {
        pub fn new(graphics_device: &GraphicsDevice) -> Self {
            let glyph_vertex_buffer = Self::build_vertex_buffer(graphics_device);
            let index_buffer = Self::build_index_buffer(graphics_device);
            let instance_buffer = Self::build_instance_buffer(graphics_device);

            let device = graphics_device.device();

            let bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[
                    // wgpu::BindGroupLayoutEntry {
                    //     binding: 0,
                    //     visibility: wgpu::ShaderStage::VERTEX,
                    //     ty: wgpu::BindingType::UniformBuffer {
                    //         dynamic: false,
                    //         min_binding_size: wgpu::BufferSize::new(64), // Size of a 4x4 f32 matrix
                    //     },
                    //     count: None,
                    // },
                    // wgpu::BindGroupLayoutEntry {
                    //     binding: 1,
                    //     visibility: wgpu::ShaderStage::FRAGMENT,
                    //     ty: wgpu::BindingType::SampledTexture {
                    //         multisampled: false,
                    //         component_type: wgpu::TextureComponentType::Float,
                    //         dimension: wgpu::TextureViewDimension::D2,
                    //     },
                    //     count: None,
                    // },
                    // wgpu::BindGroupLayoutEntry {
                    //     binding: 2,
                    //     visibility: wgpu::ShaderStage::FRAGMENT,
                    //     ty: wgpu::BindingType::Sampler { comparison: false },
                    //     count: None,
                    // },
                ],
                });

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &[
                    // wgpu::BindGroupEntry {
                    //     binding: 0,
                    //     resource: uniform_buf.as_entire_binding(),
                    // },
                    // wgpu::BindGroupEntry {
                    //     binding: 1,
                    //     resource: wgpu::BindingResource::TextureView(&texture_view),
                    // },
                    // wgpu::BindGroupEntry {
                    //     binding: 2,
                    //     resource: wgpu::BindingResource::Sampler(&sampler),
                    // },
                ],
                label: None,
            });

            let vertex_state = wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint16,
                vertex_buffers: &[
                    wgpu::VertexBufferDescriptor {
                        stride: (std::mem::size_of::<GlyphQuadVertex>()) as wgpu::BufferAddress,
                        step_mode: wgpu::InputStepMode::Vertex,
                        attributes: &[
                            // UV (vec2)
                            wgpu::VertexAttributeDescriptor {
                                format: wgpu::VertexFormat::Float2,
                                offset: 0,
                                shader_location: 0,
                            },
                        ],
                    },
                    wgpu::VertexBufferDescriptor {
                        stride: (std::mem::size_of::<GlyphInstanceData>()) as wgpu::BufferAddress,
                        step_mode: wgpu::InputStepMode::Instance,
                        attributes: &[
                            // pos (vec2)
                            wgpu::VertexAttributeDescriptor {
                                format: wgpu::VertexFormat::Float2,
                                offset: 0,
                                shader_location: 1,
                            },
                            // size (vec2)
                            wgpu::VertexAttributeDescriptor {
                                format: wgpu::VertexFormat::Float2,
                                offset: 2 * 4,
                                shader_location: 2,
                            },
                            // uv_extents (vec4)
                            wgpu::VertexAttributeDescriptor {
                                format: wgpu::VertexFormat::Float4,
                                offset: (2 * 4) + (2 * 4),
                                shader_location: 3,
                            },
                            // color (vec4)
                            wgpu::VertexAttributeDescriptor {
                                format: wgpu::VertexFormat::Float4,
                                offset: (2 * 4) + (2 * 4) + (4 * 4),
                                shader_location: 4,
                            },
                        ],
                    },
                ],
            };

            let vs_module =
                device.create_shader_module(wgpu::include_spirv!("../resources/shaders/glyph.vert.spv"));
            let fs_module =
                device.create_shader_module(wgpu::include_spirv!("../resources/shaders/glyph.frag.spv"));

            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex_stage: wgpu::ProgrammableStageDescriptor {
                    module: &vs_module,
                    entry_point: "main",
                },
                fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                    module: &fs_module,
                    entry_point: "main",
                }),
                rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: wgpu::CullMode::Front,
                    ..Default::default()
                }),
                primitive_topology: wgpu::PrimitiveTopology::TriangleStrip,
                color_states: &[wgpu::ColorStateDescriptor {
                    format: graphics_device.swap_chain_descriptor().format,
                    color_blend: wgpu::BlendDescriptor::REPLACE,
                    alpha_blend: wgpu::BlendDescriptor::REPLACE,
                    write_mask: wgpu::ColorWrite::ALL,
                }],
                depth_stencil_state: None,
                vertex_state,
                sample_count: 1,
                sample_mask: !0,
                alpha_to_coverage_enabled: false,
            });

            Self { glyph_vertex_buffer, index_buffer, instance_buffer, bind_group, pipeline }
        }

        // pub fn render(
        //     &mut self,
        //     glyph_positions: &[PositionedGlyph],
        //     glyph_texture: &Texture2d,
        //     target: &mut glium::Frame,
        //     window_size: glutin::dpi::PhysicalSize,
        // ) {
        //     if glyph_positions.len() > MAX_INSTANCE_COUNT {
        //         warn!("Trying to render more glyphs than the maximum. Max = {}, attempted render count = {}", MAX_INSTANCE_COUNT, glyph_positions.len());
        //         return;
        //     }

        //     // Update the glyph instance data.
        //     {
        //         let mut mapping = self.instance_buffer.map();
        //         for (glyph, instance) in glyph_positions.iter().zip(mapping.iter_mut()) {
        //             // Reference: https://github.com/mooman219/fontdue/issues/10#issuecomment-603480026
        //             // The Y position is "inverted" because currently fontdue assumes
        //             // the Y axis decreases as you move down the screen.
        //             instance.pos = [glyph.x, -(glyph.y + glyph.height as f32)];
        //             instance.size = [glyph.width as f32, glyph.height as f32];
        //             instance.uv_extents = [
        //                 glyph.texture_x,
        //                 glyph.texture_y,
        //                 glyph.texture_width,
        //                 glyph.texture_height,
        //             ];
        //             instance.color = glyph.color.as_array();
        //         }
        //     }

        //     // Limit our instances to the number we were told to draw.
        //     let instances = self
        //         .instance_buffer
        //         .slice(0..glyph_positions.len())
        //         .expect("Glyph instance count exceeded maximum");

        //     let draw_params =
        //         DrawParameters { blend: Blend::alpha_blending(), ..DrawParameters::default() };

        //     let proj =
        //         screen_projection_matrix(window_size.width as f32, window_size.height as f32);

        //     target
        //         .draw(
        //             (&self.glyph_vertex_buffer, instances.per_instance().unwrap()),
        //             &self.index_buffer,
        //             &self.shader,
        //             &glium::uniform! { proj: proj, glyph_texture: glyph_texture },
        //             &draw_params,
        //         )
        //         .unwrap();
        // }

        fn build_vertex_buffer(graphics_device: &GraphicsDevice) -> Buffer {
            let vertex_data = vec![
                GlyphQuadVertex { uv: [0.0, 1.0] },
                GlyphQuadVertex { uv: [0.0, 0.0] },
                GlyphQuadVertex { uv: [1.0, 0.0] },
                GlyphQuadVertex { uv: [1.0, 1.0] },
            ];

            let device = graphics_device.device();
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Glyph vertex buffer"),
                contents: bytemuck::cast_slice(&vertex_data),
                usage: wgpu::BufferUsage::VERTEX,
            })
        }

        fn build_index_buffer(graphics_device: &GraphicsDevice) -> Buffer {
            let index_data = vec![0u16, 1, 3, 2];

            let device = graphics_device.device();
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&index_data),
                usage: wgpu::BufferUsage::INDEX,
            })
        }

        fn build_instance_buffer(graphics_device: &GraphicsDevice) -> Buffer {
            let device = graphics_device.device();
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Index Buffer"),
                size: MAX_INSTANCE_COUNT as u64,
                usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
                mapped_at_creation: false,
            })
        }
    }
}
