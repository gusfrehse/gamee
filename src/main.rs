use std::time;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod mesh;
mod state;
mod texture;

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    window.set_cursor_visible(false);
    window.set_cursor_grab(true).unwrap();

    let mut state = pollster::block_on(state::State::new(&window));
    let mut curr_time = time::Instant::now();

    event_loop.run(move |event, _, control_flow| {
        // delta time calculation, the first delta_time is very wrong.
        state.delta_time = curr_time.elapsed();
        curr_time = time::Instant::now();

        if !state.input(&event) {
            match event {
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == window.id() => match event {
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        state.resize(**new_inner_size);
                    }
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    _ => {}
                },
                Event::RedrawRequested(_) => {
                    state.update();
                    match state.render() {
                        Ok(_) => {}
                        Err(e) => match e.downcast_ref::<wgpu::SurfaceError>() {
                            Some(wgpu::SurfaceError::Lost) => state.resize(state.size),
                            Some(wgpu::SurfaceError::OutOfMemory) => {
                                *control_flow = ControlFlow::Exit
                            }
                            Some(e) => eprintln!("{:?}", e),
                            None => eprintln!(
                                "I don't know what is happening, but you can be sure it's bad B)"
                            ),
                        },
                    }
                }
                Event::MainEventsCleared => {
                    window.request_redraw();
                }
                _ => {}
            }
        }
    });
}
