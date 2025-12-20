use crate::event_loop_main::run_on_main_thread_blocking;
use crate::overlay_window::OverlayWindowConfig;
use crate::platform::{Bounds, PlatformOverlayImpl, PlatformResult, WindowId};
use block2::StackBlock;
use core_foundation::date::CFTimeInterval;
use core_graphics::base::CGFloat;
use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSAnimatablePropertyContainer, NSAutoresizingMaskOptions, NSView, NSVisualEffectBlendingMode,
    NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView, NSWindow,
};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::NSRect;
use objc2_quartz_core::CATransaction;
use skia_safe::Image;
use std::time::Duration;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

pub struct MacOSPlatformOverlay;

fn get_ns_window_pointer(window: &Window) -> PlatformResult<*const NSWindow> {
    let window_handle = window
        .window_handle()
        .map_err(|e| format!("Failed to get window handle: {}", e))?;

    if let RawWindowHandle::AppKit(handle) = window_handle.as_raw() {
        let ns_view = handle.ns_view.as_ptr() as *mut NSView;
        if ns_view.is_null() {
            return Err("NSView pointer is null".into());
        }

        unsafe {
            let ns_view = &*ns_view;
            let ns_window = ns_view.window();
            match ns_window {
                Some(window) => Ok(Retained::as_ptr(&window)),
                None => Err("Failed to get NSWindow from NSView".into()),
            }
        }
    } else {
        Err("Expected AppKit window handle".into())
    }
}

fn get_ns_window_from_id(window_id: WindowId) -> PlatformResult<&'static NSWindow> {
    unsafe {
        let ns_window_ptr = window_id as *const NSWindow;
        if ns_window_ptr.is_null() {
            return Err("Invalid window pointer".into());
        }
        Ok(&*ns_window_ptr)
    }
}

impl PlatformOverlayImpl for MacOSPlatformOverlay {
    fn get_window_id(window: &Window) -> PlatformResult<WindowId> {
        get_ns_window_pointer(window).map(|ptr| ptr as WindowId)
    }

    fn set_window_bounds(window_id: WindowId, bounds: Bounds) -> PlatformResult<()> {
        run_on_main_thread_blocking(move || {
            let ns_window = get_ns_window_from_id(window_id).unwrap();
            let cg_rect: CGRect = bounds.into();
            let new_frame = NSRect::new(
                CGPoint::new(cg_rect.origin.x, cg_rect.origin.y),
                CGSize::new(cg_rect.size.width, cg_rect.size.height),
            );
            ns_window.setFrame_display(new_frame, true);
        });
        Ok(())
    }

    fn set_window_opacity(window_id: WindowId, opacity: f32) -> PlatformResult<()> {
        let ns_window = get_ns_window_from_id(window_id)?;
        ns_window.setAlphaValue(opacity as f64);
        ns_window.setOpaque(false);
        Ok(())
    }

    fn animate_window_bounds(
        window_id: WindowId,
        duration: Duration,
        bounds: Bounds,
    ) -> PlatformResult<bool> {
        let duration_seconds = duration.as_secs_f64();
        run_on_main_thread_blocking(move || {
            let ns_window = get_ns_window_from_id(window_id).unwrap();
            CATransaction::begin();
            CATransaction::setAnimationDuration(duration_seconds as CFTimeInterval);
            ns_window
                .animator()
                .setFrame_display_animate(bounds.into(), true, true);
            CATransaction::commit();
        });
        Ok(true)
    }

    fn animate_window_opacity(
        window_id: WindowId,
        duration: Duration,
        opacity: f32,
    ) -> PlatformResult<bool> {
        let duration_seconds = duration.as_secs_f64();
        run_on_main_thread_blocking(move || unsafe {
            let ns_window = get_ns_window_from_id(window_id.clone()).unwrap();
            let completion_block = StackBlock::new(move || {
                let ns_window = get_ns_window_from_id(window_id).unwrap();
                if opacity < f32::EPSILON {
                    ns_window.orderOut(None);
                }
            });
            let completion_block = completion_block.copy();

            ns_window.orderFront(None);
            CATransaction::begin();
            CATransaction::setAnimationDuration(duration_seconds as CFTimeInterval);
            CATransaction::setCompletionBlock(Some(&completion_block));
            ns_window.animator().setAlphaValue(opacity as CGFloat);
            CATransaction::commit();
        });
        Ok(true)
    }

    fn render_to_window(_image: &Image, _window_id: WindowId) -> PlatformResult<()> {
        Ok(())
    }

    fn initialize_overlay_window(
        window: &Window,
        config: &OverlayWindowConfig,
    ) -> PlatformResult<()> {
        let window_handle = window
            .window_handle()
            .map_err(|e| format!("Failed to get window handle: {}", e))?;

        if let RawWindowHandle::AppKit(handle) = window_handle.as_raw() {
            let ns_view = handle.ns_view.as_ptr() as *mut NSView;

            if ns_view.is_null() {
                return Err("NSView pointer is null".into());
            }

            unsafe {
                let ns_view = &*ns_view;
                let ns_window = ns_view.window();

                if let Some(ns_window) = ns_window {
                    ns_window.setAlphaValue(0.0);
                    let mut rect = ns_window.frame();
                    rect.origin.x = 0.0;
                    rect.origin.y = 0.0;

                    let mtm = MainThreadMarker::new().unwrap();
                    let effect_view =
                        NSVisualEffectView::initWithFrame(NSVisualEffectView::alloc(mtm), rect);
                    effect_view.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
                    effect_view.setState(NSVisualEffectState::Active);
                    effect_view.setWantsLayer(true);
                    effect_view.setAutoresizingMask(
                        NSAutoresizingMaskOptions::ViewWidthSizable
                            | NSAutoresizingMaskOptions::ViewHeightSizable,
                    );
                    if config.blur {
                        effect_view.setMaterial(NSVisualEffectMaterial::HUDWindow);
                    }

                    let layer = effect_view.layer().unwrap();
                    layer.setCornerRadius(15.0);

                    // Get the content view and add the effect view
                    if let Some(content_view) = ns_window.contentView() {
                        content_view.addSubview(&effect_view);
                    }
                } else {
                    return Err("Failed to get NSWindow from NSView".into());
                }
            }
        } else {
            return Err("Expected AppKit window handle".into());
        }

        Ok(())
    }
}
