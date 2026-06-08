#![allow(unsafe_code)]
use crate::ui_3d::View3DState;
use once_cell::sync::OnceCell;
use std::sync::Mutex;

#[repr(transparent)]
#[derive(Copy, Clone)]
struct View3DPtr(*mut View3DState);
unsafe impl Send for View3DPtr {}
unsafe impl Sync for View3DPtr {}

static FIRST_VIEW3D_STATE: OnceCell<Mutex<View3DPtr>> = OnceCell::new();

/// Called once from the 3-D view’s setup() to register its state.
pub(crate) fn _register_view3d_state(state: &mut View3DState) {
    let _ = FIRST_VIEW3D_STATE.set(Mutex::new(View3DPtr(state as *mut _)));
}

/// Borrow that state for a moment.
pub fn with_view3d_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut View3DState) -> R,
{
    // take the Mutex<View3DPtr>, unwrap(), then .0 to get the raw pointer:
    let raw: *mut View3DState = FIRST_VIEW3D_STATE
        .get()
        .expect("fly-to-pose called before a 3-D view exists")
        .lock()
        .unwrap()
        .0;

    // SAFETY: that pointer was registered at view‐setup time and lives for the whole run.
    let state: &mut View3DState = unsafe { &mut *raw };
    f(state)
}
