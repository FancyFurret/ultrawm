use crate::platform::macos::ffi::{get_window_id, notification, run_loop_mode, AXUIElementExt};
use crate::platform::macos::{
    app_is_manageable, window_is_manageable, MacOSPlatform, MacOSPlatformWindow, ObserveError,
    ObserveResult, ObserveResultExt,
};
use crate::platform::{
    EventDispatcher, PlatformError, PlatformErrorType, PlatformEvent, PlatformResult,
    PlatformWindowImpl, ProcessId, WindowId,
};
use application_services::accessibility_ui::{AXNotification, AXObserver, AXUIElement};
use application_services::{pid_t, AXError};
use core_foundation::runloop::CFRunLoop;
use core_foundation::string::CFString;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

type EventNotification = AXNotification<'static>;

struct EventListenerApp {
    pub observer: AXObserver,
    /// Notifications are only stored, not read, so that they aren't dropped
    pub _notifications: Vec<EventNotification>,
    pub window_notifications: HashMap<WindowId, Vec<EventNotification>>,
}

pub struct EventListenerAX {
    dispatcher: EventDispatcher,
    apps: HashMap<ProcessId, EventListenerApp>,

    // Used so that the callback can get a reference to self. This is necessary so that we can keep
    // track of the window notifications when new windows are created. We also can't use lifetimes
    // because then the callback would need to modify "apps", on an immutable reference to self.
    // Then if we tried to pub "apps" in a RefCell, we would no longer be able to use lifetimes.
    // Maybe there's a better way to do this?
    self_ref: Weak<RefCell<Self>>,
}

impl EventListenerAX {
    pub fn run(dispatcher: EventDispatcher) -> PlatformResult<Rc<RefCell<Self>>> {
        let listener = Rc::new(RefCell::new(Self {
            dispatcher: dispatcher.clone(),
            apps: HashMap::new(),
            self_ref: Weak::new(),
        }));

        listener.borrow_mut().self_ref = Rc::downgrade(&listener);

        for pid in MacOSPlatform::find_pids_with_windows()? {
            listener
                .borrow_mut()
                .observe_app(pid)
                .handle_observe_error()?;
        }

        Ok(listener)
    }

    fn observe_app(&mut self, pid: ProcessId) -> ObserveResult {
        if self.apps.contains_key(&pid) {
            return Ok(());
        }

        let app = AXUIElementExt::from(AXUIElement::create_application(pid as pid_t)?);
        let run_loop = CFRunLoop::get_current();

        app_is_manageable(&app)?;

        let observer = AXObserver::new(pid as pid_t)?;
        let notifications = self.get_app_notifications(&observer, &app).map_err(|_| {
            ObserveError::NotManageable("Could not get app notifications".to_string())
        })?;

        // Add the observer to the run loop
        run_loop.add_source(&observer.get_run_loop_source(), run_loop_mode::default());

        // Keep track of the observer so it doesn't get dropped
        self.apps.insert(
            pid,
            EventListenerApp {
                observer,
                _notifications: notifications,
                window_notifications: HashMap::new(),
            },
        );

        // Also add notifications for all this app's existing windows
        for window in app.windows()? {
            self.observe_window(&window)?;
        }

        Ok(())
    }

    fn unobserve_app(&mut self, pid: ProcessId) -> PlatformResult<()> {
        // According to the documentation, when an AXObserver is released, it is removed from the run loop
        // Notifications are also removed when the notifications are released
        self.apps.remove(&pid);
        Ok(())
    }

    fn observe_window(&mut self, window: &AXUIElementExt) -> ObserveResult {
        window_is_manageable(window)?;

        let id = get_window_id(&window.element).ok_or("Window has no id")?;
        let pid = window.pid()? as ProcessId;

        let listener_app = &self.apps.get(&pid).ok_or("Could not find app")?;
        if listener_app.window_notifications.contains_key(&id) {
            return Ok(());
        }

        let notifications = self
            .get_window_notifications(&listener_app.observer, &window)
            .map_err(|_| {
                ObserveError::NotManageable("Could not get window notifications".to_string())
            })?;

        let listener_app = self.apps.get_mut(&pid).ok_or("Could not find app")?;
        listener_app.window_notifications.insert(id, notifications);

        Ok(())
    }

    fn handle_event(
        &mut self,
        element: AXUIElement,
        notification: CFString,
        data: Option<MacOSPlatformWindow>,
    ) -> PlatformResult<()> {
        let element = AXUIElementExt::from(element);

        // First look at the application events. In this case element will be an application
        if notification == notification::application_activated() {
            let focused_window = element.focused_window()?;
            self.dispatcher
                .send(PlatformEvent::WindowFocused(MacOSPlatformWindow::new(
                    focused_window,
                )?));
            return Ok(());
        } else if notification == notification::application_shown() {
            for window in element.windows()? {
                if window.minimized()? {
                    continue;
                }

                let window = MacOSPlatformWindow::new(window)?;
                self.dispatcher.send(PlatformEvent::WindowShown(window));
            }
            return Ok(());
        } else if notification == notification::application_hidden() {
            for window in element.windows()? {
                if window.minimized()? {
                    continue;
                }

                let window = MacOSPlatformWindow::new(window)?;
                self.dispatcher.send(PlatformEvent::WindowHidden(window));
            }
            return Ok(());
        }

        // If it's not an application event, then it must be a window event. In this case element
        // will be a window.

        // Grab the window from the callback data if it's provided. This important for destroyed
        // windows, since once a window is destroyed, we can no longer get it's window id.
        let window = match data {
            Some(window) => window,
            None => (MacOSPlatformWindow::new(element))?,
        };

        let event = if notification == notification::focused_window_changed() {
            PlatformEvent::WindowFocused(window)
        } else if notification == notification::window_created() {
            let result = self.observe_window(&window.element);
            if result.is_err() {
                return result.handle_observe_error();
            }

            PlatformEvent::WindowCreated(window)
        } else if notification == notification::window_miniaturized() {
            PlatformEvent::WindowHidden(window)
        } else if notification == notification::window_deminiaturized() {
            PlatformEvent::WindowShown(window)
        } else if notification == notification::window_moved() {
            PlatformEvent::WindowMoved(window)
        } else if notification == notification::window_resized() {
            PlatformEvent::WindowResized(window)
        } else if notification == notification::element_destroyed() {
            PlatformEvent::WindowDestroyed(window.id())
        } else {
            println!("Unknown notification: {:?}", notification);
            return Ok(());
        };

        self.dispatcher.send(event);
        Ok(())
    }

    fn get_app_notifications(
        &self,
        observer: &AXObserver,
        app: &AXUIElementExt,
    ) -> PlatformResult<Vec<EventNotification>> {
        let d = &None;
        Ok(vec![
            self.notify(observer, app, notification::application_activated(), d)?,
            self.notify(observer, app, notification::application_shown(), d)?,
            self.notify(observer, app, notification::application_hidden(), d)?,
            self.notify(observer, app, notification::focused_window_changed(), d)?,
            self.notify(observer, app, notification::window_created(), d)?,
            self.notify(observer, app, notification::window_miniaturized(), d)?,
            self.notify(observer, app, notification::window_deminiaturized(), d)?,
            self.notify(observer, app, notification::window_moved(), d)?,
            self.notify(observer, app, notification::window_resized(), d)?,
        ])
    }

    fn get_window_notifications(
        &self,
        observer: &AXObserver,
        window: &AXUIElementExt,
    ) -> PlatformResult<Vec<EventNotification>> {
        let d = &Some(MacOSPlatformWindow::new(window.clone())?);
        Ok(vec![self.notify(
            observer,
            window,
            notification::element_destroyed(),
            d,
        )?])
    }

    fn notify(
        &self,
        observer: &AXObserver,
        element: &AXUIElementExt,
        notification: CFString,
        data: &Option<MacOSPlatformWindow>,
    ) -> PlatformResult<EventNotification> {
        let self_ref = self
            .self_ref
            .upgrade()
            .ok_or("Could not upgrade self reference")?;

        let callback = move |_: AXObserver,
                             element: AXUIElement,
                             notification: CFString,
                             window: &Option<MacOSPlatformWindow>| {
            let _ = self_ref
                .borrow_mut()
                .handle_event(element, notification, window.to_owned());
        };

        Ok(observer.add_notification(&element.element, notification, callback, data.to_owned())?)
    }

    pub fn app_launched(&mut self, pid: ProcessId) -> PlatformResult<()> {
        self.observe_app(pid).handle_observe_error()
    }

    pub fn app_terminated(&mut self, pid: ProcessId) -> PlatformResult<()> {
        self.unobserve_app(pid)?;
        Ok(())
    }
}

impl From<AXError> for PlatformError {
    fn from(error: AXError) -> Self {
        PlatformErrorType::Error(format!("AXError: {:?}", error)).into()
    }
}
