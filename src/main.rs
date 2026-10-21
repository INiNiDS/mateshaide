#[cfg(not(target_arch = "wasm32"))]
fn main() -> anyhow::Result<()> {
    use mateshaide::App;
    use winit::event_loop::{ControlFlow, EventLoop};

    tracing_subscriber::fmt::init();

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::new();
    event_loop.run_app(&mut app)?;

    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn main() {}
