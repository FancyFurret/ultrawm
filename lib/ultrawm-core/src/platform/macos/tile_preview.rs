use crate::platform::thread_lock::MainThreadLock;
use crate::platform::traits::PlatformTilePreviewImpl;
use crate::platform::PlatformResult;
use core_foundation::base::ToVoid;
use core_graphics::color::CGColor;
use icrate::block2::ConcreteBlock;
use icrate::objc2::rc::Id;
use icrate::objc2::{msg_send, msg_send_id, ClassType};
use icrate::AppKit::{
    NSAccessibility, NSAccessibilityUnknownSubrole, NSAnimatablePropertyContainer,
    NSBackingStoreBuffered, NSColor, NSViewHeightSizable, NSViewWidthSizable,
    NSVisualEffectBlendingModeBehindWindow, NSVisualEffectMaterialHUDWindow,
    NSVisualEffectStateActive, NSVisualEffectView, NSWindow, NSWindowStyleMaskBorderless,
    NSWindowStyleMaskResizable,
};
use icrate::CoreAnimation::{CALayer, CATransaction};
use icrate::Foundation::{CGPoint, CGSize, NSRect};

pub struct MacOSTilePreview {
    window: MainThreadLock<Id<NSWindow>>,
}

const ANIMATION_DURATION: f64 = 0.15;

impl PlatformTilePreviewImpl for MacOSTilePreview {
    fn new() -> PlatformResult<Self> {
        let window = MainThreadLock::new(|| make_window())?;
        Ok(Self { window })
    }

    fn show(&mut self) -> PlatformResult<()> {
        self.window.access(|w| unsafe {
            w.orderFront(None);

            CATransaction::begin();
            CATransaction::setAnimationDuration(ANIMATION_DURATION);
            w.animator().setAlphaValue(1.0);
            CATransaction::commit();
        })
    }

    fn hide(&mut self) -> PlatformResult<()> {
        self.window.access(|w| unsafe {
            let window_ref = w.clone();
            let completion_block = ConcreteBlock::new(move || {
                window_ref.orderOut(None);
            });
            let completion_block = completion_block.copy();

            CATransaction::begin();
            CATransaction::setAnimationDuration(ANIMATION_DURATION);
            CATransaction::setCompletionBlock(Some(&completion_block));
            w.animator().setAlphaValue(0.0);
            CATransaction::commit();
        })
    }

    fn move_to(&mut self, x: u32, y: u32, width: u32, height: u32) -> PlatformResult<()> {
        self.window.access(|w| unsafe {
            CATransaction::begin();
            CATransaction::setAnimationDuration(ANIMATION_DURATION);
            let frame = NSRect::new(
                CGPoint::new(x as f64, y as f64),
                CGSize::new(width as f64, height as f64),
            );
            w.animator().setFrame_display_animate(frame, true, true);
            CATransaction::commit();
        })
    }
}

fn make_window() -> Id<NSWindow> {
    unsafe {
        let rect = NSRect::new(CGPoint::new(0.0, 0.0), CGSize::new(1000.0, 1000.0));

        let window = NSWindow::initWithContentRect_styleMask_backing_defer_screen(
            NSWindow::alloc(),
            rect,
            NSWindowStyleMaskBorderless | NSWindowStyleMaskResizable,
            NSBackingStoreBuffered,
            false,
            None,
        );
        window.orderOut(None);
        window.setHasShadow(false);
        window.setOpaque(false);
        window.setBackgroundColor(Some(&NSColor::clearColor()));
        window.setAlphaValue(0.0);
        window.setLevel(8);
        window.setAccessibilitySubrole(Some(NSAccessibilityUnknownSubrole));

        let effect_view = NSVisualEffectView::initWithFrame(NSVisualEffectView::alloc(), rect);
        effect_view.setBlendingMode(NSVisualEffectBlendingModeBehindWindow);
        effect_view.setMaterial(NSVisualEffectMaterialHUDWindow);
        effect_view.setState(NSVisualEffectStateActive);
        effect_view.setWantsLayer(true);
        effect_view.setAutoresizingMask(NSViewWidthSizable | NSViewHeightSizable);

        let layer: Id<CALayer> = msg_send_id![&effect_view, layer];
        layer.setBorderWidth(2.0);
        layer.setCornerRadius(15.0);
        let border_color = CGColor::rgb(0.25, 0.25, 0.25, 1.0);
        let () = msg_send![&layer, setBorderColor: border_color.to_void()];

        let content_view = window.contentView().unwrap();
        content_view.addSubview(&effect_view);

        window
    }
}
