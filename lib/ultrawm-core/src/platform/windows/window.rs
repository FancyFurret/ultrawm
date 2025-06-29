use crate::platform::windows::WINDOW_BATCH;
use crate::platform::{
    Bounds, PlatformResult, PlatformWindowImpl, Position, ProcessId, Size, WindowId,
};
use std::mem;
use std::sync::atomic::Ordering;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, DeferWindowPos, GetForegroundWindow, GetWindowRect, GetWindowTextW,
    GetWindowThreadProcessId, IsIconic, SetForegroundWindow, SetWindowPos, ShowWindow, HDWP,
    HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SW_RESTORE,
};

#[derive(Debug)]
pub struct WindowsPlatformWindow {
    hwnd: HWND,
    cached_border_offsets: std::sync::RwLock<Option<(i32, i32, i32, i32)>>,
}

impl Clone for WindowsPlatformWindow {
    fn clone(&self) -> Self {
        Self {
            hwnd: self.hwnd,
            cached_border_offsets: std::sync::RwLock::new(None), // Reset cache on clone
        }
    }
}

unsafe impl Send for WindowsPlatformWindow {}
unsafe impl Sync for WindowsPlatformWindow {}

impl WindowsPlatformWindow {
    pub fn new(hwnd: HWND) -> PlatformResult<Self> {
        Ok(Self {
            hwnd,
            cached_border_offsets: std::sync::RwLock::new(None),
        })
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    /// Gets the visible window bounds, excluding invisible resize borders
    fn get_visible_bounds(&self) -> PlatformResult<RECT> {
        let mut rect = RECT::default();

        // Try to get the extended frame bounds (visible bounds) first
        unsafe {
            if DwmGetWindowAttribute(
                self.hwnd,
                DWMWA_EXTENDED_FRAME_BOUNDS,
                &mut rect as *mut _ as *mut _,
                mem::size_of::<RECT>() as u32,
            )
            .is_ok()
            {
                return Ok(rect);
            }
        }

        // Fall back to GetWindowRect if DwmGetWindowAttribute fails
        unsafe {
            GetWindowRect(self.hwnd, &mut rect).map_err(|_| "Could not get window bounds")?;
        }

        Ok(rect)
    }

    /// Calculates and caches the border offsets between GetWindowRect and DwmGetWindowAttribute
    /// Returns (left_offset, top_offset, right_offset, bottom_offset)
    fn get_border_offsets(&self) -> (i32, i32, i32, i32) {
        // Check if we have cached offsets
        if let Ok(cache) = self.cached_border_offsets.read() {
            if let Some(offsets) = *cache {
                return offsets;
            }
        }

        // Calculate offsets if not cached
        let offsets = self.calculate_border_offsets();

        // Cache the result
        if let Ok(mut cache) = self.cached_border_offsets.write() {
            *cache = Some(offsets);
        }

        offsets
    }

    /// Actually calculates the border offsets (separated for clarity)
    fn calculate_border_offsets(&self) -> (i32, i32, i32, i32) {
        let mut window_rect = RECT::default();
        let mut extended_rect = RECT::default();

        unsafe {
            // Get the full window rect (including invisible borders)
            if GetWindowRect(self.hwnd, &mut window_rect).is_err() {
                return (0, 0, 0, 0);
            }

            // Get the visible frame bounds
            if DwmGetWindowAttribute(
                self.hwnd,
                DWMWA_EXTENDED_FRAME_BOUNDS,
                &mut extended_rect as *mut _ as *mut _,
                size_of::<RECT>() as u32,
            )
            .is_err()
            {
                return (0, 0, 0, 0);
            }
        }

        // Calculate the border differences on all sides
        let left_offset = extended_rect.left - window_rect.left;
        let top_offset = extended_rect.top - window_rect.top;
        let right_offset = window_rect.right - extended_rect.right;
        let bottom_offset = window_rect.bottom - extended_rect.bottom;

        (left_offset, top_offset, right_offset, bottom_offset)
    }

    /// Invalidates the cached border offsets (call when window state might have changed)
    pub fn invalidate_border_cache(&self) {
        if let Ok(mut cache) = self.cached_border_offsets.write() {
            *cache = None;
        }
    }

    fn bounds_match(&self, bounds: &Bounds) -> bool {
        let current_pos = self.position();
        let current_size = self.size();
        current_pos.x == bounds.position.x
            && current_pos.y == bounds.position.y
            && current_size.width == bounds.size.width
            && current_size.height == bounds.size.height
    }
}

impl PlatformWindowImpl for WindowsPlatformWindow {
    fn id(&self) -> WindowId {
        self.hwnd.0 as WindowId
    }

    fn pid(&self) -> ProcessId {
        let mut pid = 0;
        unsafe {
            GetWindowThreadProcessId(self.hwnd, Some(&mut pid));
        }
        pid as ProcessId
    }

    fn title(&self) -> String {
        let mut text: [u16; 512] = [0; 512];
        let len = unsafe { GetWindowTextW(self.hwnd, &mut text) };

        String::from_utf16_lossy(&text[..len as usize])
    }

    fn position(&self) -> Position {
        let rect = self
            .get_visible_bounds()
            .expect("Could not get window position");
        Position {
            x: rect.left,
            y: rect.top,
        }
    }

    fn size(&self) -> Size {
        let rect = self
            .get_visible_bounds()
            .expect("Could not get window size");
        Size {
            width: (rect.right - rect.left) as u32,
            height: (rect.bottom - rect.top) as u32,
        }
    }

    fn visible(&self) -> bool {
        unsafe { !IsIconic(self.hwnd).as_bool() }
    }

    fn set_bounds(&self, bounds: &Bounds) -> PlatformResult<()> {
        // Skip if bounds haven't changed to avoid unnecessary operations
        if self.bounds_match(bounds) {
            return Ok(());
        }

        let hdswp = WINDOW_BATCH.load(Ordering::Relaxed);

        let (left_offset, top_offset, right_offset, bottom_offset) = self.get_border_offsets();
        let adjusted_x = bounds.position.x - left_offset;
        let adjusted_y = bounds.position.y - top_offset;
        let adjusted_width = bounds.size.width as i32 + left_offset + right_offset;
        let adjusted_height = bounds.size.height as i32 + top_offset + bottom_offset;

        let flags = SWP_NOZORDER | SWP_NOACTIVATE;
        if hdswp > 0 {
            // flags |= SWP_NOCOPYBITS;
        }

        unsafe {
            if hdswp > 0 {
                DeferWindowPos(
                    HDWP(hdswp as *mut _),
                    self.hwnd,
                    None,
                    adjusted_x,
                    adjusted_y,
                    adjusted_width,
                    adjusted_height,
                    flags,
                )
                .unwrap();
            } else {
                SetWindowPos(
                    self.hwnd,
                    None,
                    adjusted_x,
                    adjusted_y,
                    adjusted_width,
                    adjusted_height,
                    flags,
                )
                .unwrap();
            }
        }

        Ok(())
    }

    /// Windows 11 makes this more difficult than it needs to be...
    /// https://www.reddit.com/r/dotnet/comments/1da3uec/issue_with_setforegroundhandle_with_windows_11/
    fn focus(&self) -> PlatformResult<()> {
        unsafe {
            // First, restore the window if it's minimized
            ShowWindow(self.hwnd, SW_RESTORE)
                .ok()
                .map_err(|e| format!("Failed to restore window: {}", e))?;

            // Get thread IDs
            let current_thread_id = GetCurrentThreadId();
            let foreground_window = GetForegroundWindow();
            let foreground_thread_id = GetWindowThreadProcessId(foreground_window, None);
            let dest_thread_id = GetWindowThreadProcessId(self.hwnd, None);

            let attached_current = if current_thread_id != dest_thread_id {
                AttachThreadInput(current_thread_id, dest_thread_id, true).as_bool()
            } else {
                false // Don't need to attach to ourselves
            };

            let attached_foreground = if foreground_thread_id != dest_thread_id {
                AttachThreadInput(foreground_thread_id, dest_thread_id, true).as_bool()
            } else {
                false // Don't need to attach if already the same thread
            };

            // Bring window to foreground
            BringWindowToTop(self.hwnd)
                .map_err(|e| format!("Failed to bring window to top: {}", e))?;
            SetForegroundWindow(self.hwnd)
                .ok()
                .map_err(|e| format!("Failed to set foreground window: {}", e))?;

            // Detach threads
            if attached_current {
                AttachThreadInput(current_thread_id, dest_thread_id, false)
                    .ok()
                    .map_err(|e| format!("Failed to detach thread: {}", e))?;
            }
            if attached_foreground {
                AttachThreadInput(foreground_thread_id, dest_thread_id, false)
                    .ok()
                    .map_err(|e| format!("Failed to detach thread: {}", e))?;
            }
        }
        Ok(())
    }

    fn set_always_on_top(&self, always_on_top: bool) -> PlatformResult<()> {
        unsafe {
            let hwnd_insert_after = if always_on_top {
                Some(HWND_TOPMOST)
            } else {
                Some(HWND_NOTOPMOST)
            };

            SetWindowPos(
                self.hwnd,
                hwnd_insert_after,
                0, // x - ignored due to SWP_NOMOVE
                0, // y - ignored due to SWP_NOMOVE
                0, // width - ignored due to SWP_NOSIZE
                0, // height - ignored due to SWP_NOSIZE
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            )
            .map_err(|e| format!("Failed to set always on top: {}", e))?;
        }
        Ok(())
    }
}
