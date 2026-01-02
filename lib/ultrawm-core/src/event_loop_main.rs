use crate::{UltraWMFatalError, UltraWMResult};
use std::cell::RefCell;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus};
use winit::window::{Window, WindowId};

#[allow(clippy::type_complexity)]
pub enum MainThreadMessage {
    RunOnMainThread {
        task: Box<dyn FnOnce() + Send>,
    },
    GetEventLoop {
        task: Box<dyn FnOnce(&ActiveEventLoop) + Send>,
    },
    Shutdown,
    PanicError {
        message: String,
    },
}

pub(crate) static MAIN_THREAD_TASK_SENDER: std::sync::OnceLock<Sender<MainThreadMessage>> =
    std::sync::OnceLock::new();

static EVENT_LOOP_PROXY: std::sync::OnceLock<EventLoopProxy<MainThreadMessage>> =
    std::sync::OnceLock::new();

thread_local! {
    static MAIN_THREAD_TASK_RECEIVER: RefCell<Option<Receiver<MainThreadMessage>>> = RefCell::new(None);
}

pub async fn run_on_main_thread<F, R>(f: F) -> R
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let s = MAIN_THREAD_TASK_SENDER
        .get()
        .expect("Event loop has not been started yet");
    let (result_tx, result_rx) = tokio::sync::oneshot::channel::<R>();

    let task = Box::new(move || {
        let result = f();
        let _ = result_tx.send(result);
    });

    s.send(MainThreadMessage::RunOnMainThread { task })
        .expect("Failed to send task to main thread");

    result_rx
        .await
        .expect("run_on_main_thread task was cancelled")
}

pub fn run_on_main_thread_blocking<F, R>(f: F) -> R
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let s = MAIN_THREAD_TASK_SENDER
        .get()
        .expect("Event loop has not been started yet");

    let (result_tx, result_rx) = std::sync::mpsc::channel::<R>();

    let task = Box::new(move || {
        let result = f();
        let _ = result_tx.send(result);
    });

    s.send(MainThreadMessage::RunOnMainThread { task })
        .expect("Failed to send task to main thread");

    result_rx
        .recv()
        .expect("run_on_main_thread task was cancelled")
}

pub fn get_event_loop_blocking<F, R>(f: F) -> R
where
    F: FnOnce(&ActiveEventLoop) -> R + Send + 'static,
    R: Send + 'static,
{
    let proxy = EVENT_LOOP_PROXY
        .get()
        .expect("Event loop has not been started yet");

    let (result_tx, result_rx) = std::sync::mpsc::channel::<R>();

    let task = Box::new(move |e: &ActiveEventLoop| {
        let result = f(e);
        let _ = result_tx.send(result);
    });

    proxy
        .send_event(MainThreadMessage::GetEventLoop { task })
        .unwrap_or_else(|_| panic!("Failed to send event loop"));

    result_rx
        .recv()
        .expect("get_event_loop_blocking task was cancelled")
}

enum TaskResult {
    Continue,
    Shutdown,
    Panic(String),
}

pub struct EventLoopMain {}

impl EventLoopMain {
    pub fn run() -> UltraWMResult<()> {
        // Create the event loop with our custom user-event type.
        let mut event_loop = EventLoop::with_user_event().build().unwrap();

        EVENT_LOOP_PROXY
            .set(event_loop.create_proxy())
            .map_err(|_| "Event loop proxy already initialized")?;

        let (task_tx, task_rx) = channel();
        MAIN_THREAD_TASK_SENDER
            .set(task_tx)
            .map_err(|_| "Main thread tasks already initialized")?;

        MAIN_THREAD_TASK_RECEIVER.with(|cell| {
            *cell.borrow_mut() = Some(task_rx);
        });

        event_loop.set_control_flow(ControlFlow::Wait);
        let mut app = App::default();
        let mut panic_error: Option<String> = None;

        loop {
            let exit = event_loop.pump_app_events(Some(Duration::from_millis(100)), &mut app);
            if matches!(exit, PumpStatus::Exit(_)) {
                break;
            }

            // Check for main thread tasks after waking up
            match Self::process_main_thread_tasks() {
                TaskResult::Continue => {}
                TaskResult::Shutdown => break,
                TaskResult::Panic(msg) => {
                    crate::shutdown();
                    panic_error = Some(msg);
                    break;
                }
            }
        }

        // If we exited due to a panic, return an error
        if let Some(msg) = panic_error {
            return Err(UltraWMFatalError::Error(msg));
        }

        Ok(())
    }

    fn process_main_thread_tasks() -> TaskResult {
        MAIN_THREAD_TASK_RECEIVER.with(|cell| {
            if let Some(task_rx) = cell.borrow_mut().as_mut() {
                while let Ok(message) = task_rx.try_recv() {
                    match message {
                        MainThreadMessage::RunOnMainThread { task } => {
                            task();
                        }
                        MainThreadMessage::Shutdown => {
                            return TaskResult::Shutdown;
                        }
                        MainThreadMessage::PanicError { message } => {
                            return TaskResult::Panic(message);
                        }
                        _ => (),
                    }
                }
            }
            TaskResult::Continue
        })
    }

    pub fn shutdown() {
        if let Some(s) = MAIN_THREAD_TASK_SENDER.get() {
            let _ = s.send(MainThreadMessage::Shutdown);
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
            MainThreadMessage::GetEventLoop { task } => {
                task(event_loop);
            }
            _ => (),
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            _ => (),
        }
    }
}
