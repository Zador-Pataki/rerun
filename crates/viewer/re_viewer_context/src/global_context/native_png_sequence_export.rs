use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use crate::ViewId;

pub const NATIVE_PNG_SEQUENCE_EXPORT_READBACK_ID: re_renderer::GpuReadbackIdentifier =
    0x5653_464d_3350_4e47;

#[derive(Clone, Debug)]
pub struct NativePngSequenceExportFrameRequest {
    pub frame_index: u64,
    pub output_path: PathBuf,
    scheduled_view_count: Arc<AtomicUsize>,
}

impl NativePngSequenceExportFrameRequest {
    pub fn new(frame_index: u64, output_path: PathBuf) -> Self {
        Self {
            frame_index,
            output_path,
            scheduled_view_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn mark_scheduled(&self) {
        self.scheduled_view_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn scheduled_view_count(&self) -> usize {
        self.scheduled_view_count.load(Ordering::Relaxed)
    }
}

#[derive(Clone, Debug)]
pub struct NativePngSequenceExportReadback {
    pub frame_index: u64,
    pub output_path: PathBuf,
    pub view_id: ViewId,
}
