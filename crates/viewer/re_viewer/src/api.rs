#![allow(unsafe_code)]

use once_cell::sync::OnceCell;
use std::sync::Mutex;

use re_view_spatial::ui_3d::View3DState;

// A thin, Send+Sync wrapper around the raw pointer so it can live in `OnceCell`.
#[repr(transparent)]
#[derive(Copy, Clone)]
struct View3DPtr(*mut View3DState);
unsafe impl Send for View3DPtr {}
unsafe impl Sync for View3DPtr {}

static FIRST_VIEW3D_STATE: OnceCell<Mutex<View3DPtr>> = OnceCell::new();

/// Register the *first* 3-D–view state that is created.
///
/// Called once from `ui.rs`, ignored on subsequent calls.
pub fn _register_view3d_state(state: &mut View3DState) {
    let _ = FIRST_VIEW3D_STATE.set(Mutex::new(View3DPtr(state as *mut _)));
}

/// Borrow that `View3DState` mutably for the duration of `f`.
pub fn with_view3d_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut View3DState) -> R,
{
    let ptr = FIRST_VIEW3D_STATE
        .get()
        .expect("fly-to-pose called before any 3-D view exists")
        .lock()
        .unwrap()
        .0;                                      // just the raw pointer

    // SAFETY: the `View3DState` lives for the whole program (owned by egui/eframe).
    let state: &mut View3DState = unsafe { &mut *ptr };
    f(state)
}
