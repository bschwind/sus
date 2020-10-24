use wgpu::{BackendBit, Instance};
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

const CORNFLOWER_BLUE: wgpu::Color =
    wgpu::Color { r: 100.0 / 255.0, g: 149.0 / 255.0, b: 237.0 / 255.0, a: 1.0 };

async fn run() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().with_title("sus").build(&event_loop).unwrap();

    let size = window.inner_size();

    // All the apis that wgpu offers first tier of support for (Vulkan + Metal + DX12 + Browser WebGPU).
    let instance = Instance::new(BackendBit::PRIMARY);
    let surface = unsafe { instance.create_surface(&window) };
    let swapchain_format = wgpu::TextureFormat::Bgra8UnormSrgb;

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
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
                shader_validation: true,
            },
            None,
        )
        .await
        .expect("Failed to create device");

    let mut swapchain_desc = wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        format: swapchain_format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Mailbox,
    };

    let mut swap_chain = device.create_swap_chain(&surface, &swapchain_desc);

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent { event: WindowEvent::Resized(_size), .. } => {
                // Recreate the swap chain with the new size
                swapchain_desc.width = size.width;
                swapchain_desc.height = size.height;
                swap_chain = device.create_swap_chain(&surface, &swapchain_desc);
            },
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                },
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(virtual_code),
                            state: ElementState::Pressed,
                            ..
                        },
                    ..
                } => match virtual_code {
                    VirtualKeyCode::Escape => {
                        *control_flow = ControlFlow::Exit;
                    },
                    _ => (),
                },
                _ => (),
            },
            Event::RedrawRequested(_window_id) => {
                // Draw the scene
                let frame = swap_chain
                    .get_current_frame()
                    .expect("Failed to acquire next swap chain texture")
                    .output;
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                {
                    let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                            attachment: &frame.view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(CORNFLOWER_BLUE),
                                store: true,
                            },
                        }],
                        depth_stencil_attachment: None,
                    });
                }

                queue.submit(Some(encoder.finish()));
            },
            _ => (),
        }
    });
}

fn main() {
    futures::executor::block_on(run());
}
