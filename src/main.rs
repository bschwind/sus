use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    let event_loop = EventLoop::new();
    let _window = WindowBuilder::new().with_title("sus").build(&event_loop).unwrap();

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent { event: WindowEvent::Resized(_size), .. } => {
                // Recreate the swap chain with the new size
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
            },
            _ => (),
        }
    });
}
