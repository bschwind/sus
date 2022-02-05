use crate::SusGame;
use sus_common::{
    components::player::PlayerId,
    simple_game::{
        bevy::{
            App, Commands, IntoSystem, ParallelSystemDescriptorCoercion, Plugin, Query, Res,
            ResMut, Transform,
        },
        graphics::{
            text::{AxisAlign, Color, DefaultFont, StyledText, TextAlignment, TextSystem},
            DebugDrawer, FullscreenQuad, GraphicsDevice,
        },
        wgpu,
    },
    systems::labels,
    PlayerInput,
};

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup.system()).add_system(render.system().label(labels::Render));
    }
}

fn setup(mut commands: Commands, graphics_device: Res<GraphicsDevice>) {
    let text_system: TextSystem = TextSystem::new(&graphics_device);
    let debug_drawer = DebugDrawer::new(&graphics_device);
    let fullscreen_quad = FullscreenQuad::new(&graphics_device);
    let player_input = PlayerInput::default();

    commands.insert_resource(text_system);
    commands.insert_resource(debug_drawer);
    commands.insert_resource(fullscreen_quad);
    commands.insert_resource(player_input);
}

fn render(
    game: Res<SusGame>,
    mut graphics_device: ResMut<GraphicsDevice>,
    fullscreen_quad: ResMut<FullscreenQuad>,
    mut text_system: ResMut<TextSystem>,
    mut debug_drawer: ResMut<DebugDrawer>,
    players: Query<(&PlayerId, &Transform)>,
) {
    let mut frame_encoder = graphics_device.begin_frame();

    {
        let encoder = &mut frame_encoder.encoder;

        let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Screen Clear"),
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: &frame_encoder.backbuffer_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });
    }

    fullscreen_quad.render(&mut frame_encoder);

    let mut shape_recorder = debug_drawer.begin();

    for (_player_id, transform) in players.iter() {
        shape_recorder.draw_circle(transform.translation, 2.0, 0.0);
    }
    shape_recorder.end(&mut frame_encoder);

    text_system.render_horizontal(
        TextAlignment {
            x: AxisAlign::Start(10),
            y: AxisAlign::WindowCenter,
            max_width: None,
            max_height: None,
        },
        &[
            StyledText::default_styling("This is a test."),
            StyledText {
                text: "Another test, blue this time",
                font: DefaultFont::SpaceMono400(40),
                color: Color::new(0, 0, 255, 255),
            },
            StyledText {
                text: "\nTest with a line break, green.",
                font: DefaultFont::SpaceMono400(40),
                color: Color::new(0, 255, 0, 255),
            },
            StyledText {
                text: "Red test\nHere are some numbers:\n0123456789!@#$%^&*(){}[].",
                font: DefaultFont::SpaceMono400(40),
                color: Color::new(255, 0, 0, 255),
            },
            StyledText {
                text: "\nOpacity test, this should be half-faded white",
                font: DefaultFont::SpaceMono400(40),
                color: Color::new(255, 255, 255, 128),
            },
            StyledText {
                text: &format!(
                    "\nServer addr: {}\nConnected: {}",
                    game.server_addr, game.connected
                ),
                font: DefaultFont::SpaceMono400(40),
                color: Color::new(255, 255, 255, 255),
            },
        ],
        &mut frame_encoder,
    );

    frame_encoder.finish();
}
