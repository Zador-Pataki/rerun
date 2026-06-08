use std::path::PathBuf;

use re_entity_db::EntityDb;
use re_log_types::{TimeInt, TimelineName};
use re_renderer::ScreenshotProcessor;
use re_viewer_context::{
    NativePngSequenceExportFrameRequest, NativePngSequenceExportReadback, RecordingConfig,
    NATIVE_PNG_SEQUENCE_EXPORT_READBACK_ID,
};

#[derive(Clone)]
pub struct NativePngSequenceExportOptions {
    pub output_dir: PathBuf,
    pub timeline_name: TimelineName,
}

#[derive(Default)]
pub struct NativePngSequenceExport {
    options: Option<NativePngSequenceExportOptions>,
    frame_times: Option<Vec<TimeInt>>,
    total_frame_count: usize,
    next_frame_index: usize,
    waiting_for_frame: Option<u64>,
    unscheduled_frame_attempts: usize,
    warmup_frames_remaining: usize,
    observed_frame_count: usize,
    stable_frame_count_observations: usize,
    timeline_wait_frames: usize,
    done: bool,
    failed: bool,
}

const REQUIRED_STABLE_FRAME_COUNT_OBSERVATIONS: usize = 60;
const MAX_TIMELINE_WAIT_FRAMES: usize = 1800;

impl NativePngSequenceExport {
    pub fn new(options: Option<NativePngSequenceExportOptions>) -> Self {
        let warmup_frames_remaining = if options.is_some() { 3 } else { 0 };
        Self {
            options,
            frame_times: None,
            total_frame_count: 0,
            next_frame_index: 0,
            waiting_for_frame: None,
            unscheduled_frame_attempts: 0,
            warmup_frames_remaining,
            observed_frame_count: 0,
            stable_frame_count_observations: 0,
            timeline_wait_frames: 0,
            done: false,
            failed: false,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.options.is_some()
    }

    pub fn should_close(&self) -> bool {
        self.done || self.failed
    }

    pub fn prepare_frame(
        &mut self,
        recording: &EntityDb,
        rec_cfg: &mut RecordingConfig,
    ) -> Option<NativePngSequenceExportFrameRequest> {
        let options = self.options.clone()?;
        if self.done || self.failed || self.waiting_for_frame.is_some() {
            return None;
        }
        if self.warmup_frames_remaining > 0 {
            self.warmup_frames_remaining -= 1;
            return None;
        }
        if self.frame_times.is_none() {
            let Some(frame_times) =
                self.collect_stable_frame_times(recording, &options.timeline_name)
            else {
                return None;
            };
            re_log::info!(
                "Exporting {} Spatial3D PNG frames to {:?}",
                frame_times.len(),
                options.output_dir
            );
            self.total_frame_count = frame_times.len();
            self.frame_times = Some(frame_times);
            if let Err(err) = prepare_output_dir(&options.output_dir) {
                return self.fail(err);
            }
        }

        let frame_times = self.frame_times.as_ref()?;
        if self.next_frame_index >= frame_times.len() {
            self.done = true;
            re_log::info!(
                "Finished Spatial3D PNG sequence export to {:?}",
                options.output_dir
            );
            return None;
        }

        let frame_index = self.next_frame_index as u64;
        let frame_time = frame_times[self.next_frame_index];
        let timeline = recording
            .timelines()
            .get(&options.timeline_name)
            .copied()
            .unwrap_or_else(|| re_log_types::Timeline::new_sequence(options.timeline_name));
        {
            let mut time_ctrl = rec_cfg.time_ctrl.write();
            time_ctrl.set_timeline_and_time(timeline, frame_time);
            time_ctrl.pause();
        }

        self.waiting_for_frame = Some(frame_index);
        Some(NativePngSequenceExportFrameRequest::new(
            frame_index,
            options
                .output_dir
                .join(format!("frame_{frame_index:06}.png")),
        ))
    }

    pub fn validate_frame_request(&mut self, request: &NativePngSequenceExportFrameRequest) {
        if self.done || self.failed || self.waiting_for_frame != Some(request.frame_index) {
            return;
        }

        match request.scheduled_view_count() {
            1 => {
                self.unscheduled_frame_attempts = 0;
            }
            0 => {
                self.waiting_for_frame = None;
                self.unscheduled_frame_attempts += 1;
                if self.unscheduled_frame_attempts > 60 {
                    self.fail::<()>(format!(
                        "Cannot export Spatial3D PNG frame {}: no 3D spatial view scheduled a screenshot",
                        request.frame_index
                    ));
                }
            }
            count => {
                self.waiting_for_frame = None;
                self.fail::<()>(format!(
                    "Cannot export Spatial3D PNG frame {}: {count} 3D spatial views scheduled screenshots; expected exactly one",
                    request.frame_index
                ));
            }
        }
    }

    pub fn drain_readbacks(&mut self, render_ctx: &re_renderer::RenderContext) {
        if self.failed {
            return;
        }
        loop {
            let mut save_failed = false;
            let readback_available =
                ScreenshotProcessor::next_readback_result::<NativePngSequenceExportReadback>(
                    render_ctx,
                    NATIVE_PNG_SEQUENCE_EXPORT_READBACK_ID,
                    |rgba, extent, readback| {
                        if let Err(err) = image::save_buffer(
                            &readback.output_path,
                            rgba,
                            extent.x,
                            extent.y,
                            image::ColorType::Rgba8,
                        ) {
                            save_failed = true;
                            re_log::error!(
                                "Failed to save Spatial3D PNG export frame {:?}: {err}",
                                readback.output_path
                            );
                        } else {
                            re_log::debug!(
                                "Saved Spatial3D PNG export frame {} from view {:?} to {:?}",
                                readback.frame_index,
                                readback.view_id,
                                readback.output_path
                            );
                        }
                    },
                )
                .is_some();

            if !readback_available {
                break;
            }

            if save_failed {
                self.waiting_for_frame = None;
                self.failed = true;
                break;
            } else if self.waiting_for_frame.is_some() {
                self.waiting_for_frame = None;
                self.next_frame_index += 1;
                if self.next_frame_index == self.total_frame_count
                    || self.next_frame_index == 1
                    || self.next_frame_index % 25 == 0
                {
                    re_log::info!(
                        "Exported Spatial3D PNG frames: {}/{}",
                        self.next_frame_index,
                        self.total_frame_count
                    );
                }
            }
        }
    }

    fn fail<T>(&mut self, message: impl Into<String>) -> Option<T> {
        let message = message.into();
        self.failed = true;
        re_log::error!("{message}");
        None
    }

    fn collect_stable_frame_times(
        &mut self,
        recording: &EntityDb,
        timeline_name: &TimelineName,
    ) -> Option<Vec<TimeInt>> {
        self.timeline_wait_frames += 1;
        let frame_times = match collect_frame_times(recording, timeline_name) {
            Ok(frame_times) => frame_times,
            Err(err) => {
                if self.timeline_wait_frames > MAX_TIMELINE_WAIT_FRAMES {
                    return self.fail(err);
                }
                if self.timeline_wait_frames == 1 || self.timeline_wait_frames % 120 == 0 {
                    re_log::info!(
                        "Waiting for Spatial3D PNG export timeline {timeline_name:?} to load"
                    );
                }
                return None;
            }
        };

        let frame_count = frame_times.len();
        if frame_count != self.observed_frame_count {
            self.observed_frame_count = frame_count;
            self.stable_frame_count_observations = 0;
            re_log::info!(
                "Waiting for Spatial3D PNG export timeline {timeline_name:?} to stabilize: {frame_count} frames loaded"
            );
            return None;
        }

        self.stable_frame_count_observations += 1;
        if self.stable_frame_count_observations < REQUIRED_STABLE_FRAME_COUNT_OBSERVATIONS {
            return None;
        }

        Some(frame_times)
    }
}

fn collect_frame_times(
    recording: &EntityDb,
    timeline_name: &TimelineName,
) -> Result<Vec<TimeInt>, String> {
    let timelines = recording.times_per_timeline();
    let Some(stats) = timelines.get(timeline_name) else {
        return Err(format!(
            "Cannot export Spatial3D PNG sequence: timeline {timeline_name:?} was not found"
        ));
    };
    let frame_times = stats.per_time.keys().copied().collect::<Vec<_>>();
    if frame_times.is_empty() {
        return Err(format!(
            "Cannot export Spatial3D PNG sequence: timeline {timeline_name:?} has no frames"
        ));
    }
    Ok(frame_times)
}

fn prepare_output_dir(output_dir: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(output_dir).map_err(|err| {
        format!("Failed to create Spatial3D PNG export directory {output_dir:?}: {err}")
    })?;
    Ok(())
}
