//! Files dropped onto the window on native Wayland (Linux). winit 0.30 delivers no drops there
//! (its #4729 is not released), so `photocraft-wayland-drop` listens on winit's own connection and
//! the app routes what it reports like any other drop (`PhotocraftApp::open_dropped`).
//!
//! Nothing starts unless winit is on Wayland. The listener stops when the service is dropped,
//! which `PhotocraftApp::on_exit` does before winit closes its connection (the safety argument of
//! `photocraft_wayland_drop::linux::adopt`).

use photocraft_ui_egui::WaylandDropsFn;

/// The drop service for the window `cc` created, or `None` when it is not on Wayland or the
/// compositor offers no drag and drop (the window then shows how else to open files).
pub fn service(cc: &eframe::CreationContext<'_>) -> Option<WaylandDropsFn> {
    use eframe::wgpu::rwh::{HasDisplayHandle, RawDisplayHandle};
    let RawDisplayHandle::Wayland(handle) = cc.display_handle().ok()?.as_raw() else { return None };
    let ctx = cc.egui_ctx.clone();
    let Some((listener, drops)) = photocraft_wayland_drop::Listener::start(handle.display, move || ctx.request_repaint()) else {
        log::warn!("decision: drop: native Wayland file drops could not be set up (no data device on this compositor); File › Open and paste still work");
        return None;
    };
    log::info!("drop: listening for files dropped on the window (native Wayland)");
    Some(Box::new(move || {
        let _keep = &listener; // dropping the service stops the listener
        drops
            .try_iter()
            .filter_map(|d| match d {
                photocraft_wayland_drop::Drop::Files { paths, at } => Some((paths, Some(eframe::egui::pos2(at.0 as f32, at.1 as f32)))),
                photocraft_wayland_drop::Drop::NotFiles => {
                    log::info!("drop: something other than files was dropped (no text/uri-list); nothing opened");
                    None
                }
            })
            .collect()
    }))
}
