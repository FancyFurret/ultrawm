use crate::UltraWMResult;
use log::trace;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::window::{Window, WindowId};

#[allow(clippy::type_complexity)]
pub enum MainThreadMessage {
    RunOnMainThread {
        task: Box<dyn FnOnce(&ActiveEventLoop) + Send>,
    },
    Shutdown,
}

pub static EVENT_LOOP_PROXY: std::sync::OnceLock<EventLoopProxy<MainThreadMessage>> =
    std::sync::OnceLock::new();

pub async fn run_on_main_thread<F, R>(f: F) -> R
where
    F: FnOnce(&ActiveEventLoop) -> R + Send + 'static,
    R: Send + 'static,
{
    // Ensure we have the proxy
    let proxy = EVENT_LOOP_PROXY
        .get()
        .expect("Event loop has not been started yet")
        .clone();

    // Channel to get the result back.
    let (tx, rx) = tokio::sync::oneshot::channel::<R>();

    // Wrap the user closure so that it executes on the main thread and sends
    // the result back.
    let task = Box::new(move |event_loop: &ActiveEventLoop| {
        let result = f(event_loop);
        let _ = tx.send(result);
    });

    // Send the message – ignore errors because they can only happen if the
    // event loop already exited.
    let _ = proxy.send_event(MainThreadMessage::RunOnMainThread { task });

    // Await the response.
    rx.await.expect("run_on_main_thread task was cancelled")
}

pub fn run_on_main_thread_blocking<F, R>(f: F) -> R
where
    F: FnOnce(&ActiveEventLoop) -> R + Send + 'static,
    R: Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel();
    let proxy = EVENT_LOOP_PROXY
        .get()
        .expect("Event loop has not been started yet")
        .clone();

    let task = Box::new(move |event_loop: &ActiveEventLoop| {
        let result = f(event_loop);
        let _ = tx.send(result);
    });

    let _ = proxy.send_event(MainThreadMessage::RunOnMainThread { task });
    rx.recv().expect("run_on_main_thread task was cancelled")
}

pub struct EventLoopMain {}

impl EventLoopMain {
    pub fn run() -> UltraWMResult<()> {
        // Create the event loop with our custom user-event type.
        let event_loop = EventLoop::with_user_event().build().unwrap();
        // Store the proxy so that other threads can send messages.
        EVENT_LOOP_PROXY
            .set(event_loop.create_proxy())
            .map_err(|_| "Event loop proxy already initialized")?;
        event_loop.set_control_flow(ControlFlow::Wait);

        let mut app = App::default();
        event_loop
            .run_app(&mut app)
            .map_err(|_| "Failed to start event loop")?;
        Ok(())
    }

    pub fn shutdown() {
        if let Some(proxy) = EVENT_LOOP_PROXY.get() {
            let _ = proxy.send_event(MainThreadMessage::Shutdown);
        }
    }
}

struct ActiveAnimation {
    mutator: Box<dyn FnMut() -> bool + Send>,
}

#[derive(Default)]
struct App {
    window: Option<Window>,
    animators: Vec<ActiveAnimation>,
}

impl ApplicationHandler<MainThreadMessage> for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: MainThreadMessage) {
        match event {
            MainThreadMessage::RunOnMainThread { task } => {
                task(event_loop);
            }
            MainThreadMessage::Shutdown => {
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                trace!("The close button was pressed; stopping");
                event_loop.exit();
            }
            _ => (),
        }
    }
}
