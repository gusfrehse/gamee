use std::time;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod camera;
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
        state.delta_time = curr_time.elapsed();
        curr_time = time::Instant::now();

        *control_flow = state.input(&event);
        match event {
            Event::MainEventsCleared => {
                state.update();
                match state.render() {
                    Ok(_) => {}
                    Err(e) => match e.downcast_ref::<wgpu::SurfaceError>() {
                        Some(wgpu::SurfaceError::Lost) => state.resize(state.size),
                        Some(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                        Some(e) => eprintln!("{:?}", e),
                        None => eprintln!(
                            "I don't know what is happening, but you can be sure it's bad B)"
                        ),
                    },
                }
            }
            _ => {}
        }
    });
}
