# VideoSfM Camera Pyramid Style Status

Date: 2026-05-26

## Implemented behavior

- Generic Rerun `Pinhole` archetypes now accept optional `Color` and `Radius` components.
- `CamerasVisualizer` queries resolved `Color` and `Radius` with blueprint/default/fallback handling and applies them to the generated camera pyramid/frustum line batch.
- Fallbacks preserve the old default gray camera pyramid color and use `Radius::ONE_UI_POINTS`.
- Selection-panel visualizer overrides expose camera pyramid styling for camera/pinhole entities.

## UI path

Select a camera/pinhole entity or group in the viewer, then use:

`Selection panel -> Visualizers -> Cameras/Pinhole -> Camera pyramid style -> Color / Radius`

The UI group is implemented in `crates/viewer/re_selection_panel/src/visualizer_ui.rs` under `Camera pyramid style`.

## API contract

Rust:

```rust
rr.log(
    "world/cam0",
    &rerun::archetypes::Pinhole::from_focal_length_and_resolution([500.0, 500.0], [640.0, 480.0])
        .with_image_plane_distance(0.25)
        .with_color([255, 80, 40])
        .with_radius(rerun::components::Radius::new_ui_points(4.0)),
)?;
```

Python:

```python
rr.log(
    "world/cam0",
    rr.Pinhole(
        focal_length=[500.0, 500.0],
        resolution=[640.0, 480.0],
        image_plane_distance=0.25,
        color=[255, 80, 40],
        radius=rr.Radius.ui_points(4.0),
    ),
)
```

Existing recordings without these components continue to use viewer fallbacks. Viewer-side blueprint overrides can change `Color` and `Radius` without regenerating the `.rrd`.

## Proof files

- Recording: `/tmp/rerun-camera-pyramid-style.rrd`
- Baseline screenshot: `/tmp/rerun-camera-pyramid-style.png`
- Blueprint override: `/tmp/rerun-camera-pyramid-style-override.rbl`
- Override screenshot: `/tmp/rerun-camera-pyramid-style-override.png`
- Camera-only patch: `/tmp/rerun_camera_pyramid_style_feature.patch`

The screenshot command wrote the PNGs, then hit the known shutdown panic: `Failed to take store hub from the Viewer`.

Blueprint override proof:

- `/tmp/rerun-camera-pyramid-style-override.rbl` contains an override at `view/00000000-0000-0000-0000-000000000002/ViewContents/overrides/world/cam/red_thick`.
- `rrd print` showed override columns `[Color Radius]` at that path.
- Selection-panel edits call `save_blueprint_component`; `CamerasVisualizer` consumes resolved `get_mono::<Color>()` and `get_mono::<Radius>()`.

## Tests

Passed after the camera changes:

- `cargo check -p re_selection_panel`
- `cargo check -p re_view_spatial`
- `cargo test -p re_types --test types pinhole`
- `cargo test -p re_selection_panel --features re_viewer_context/testing visualizer_ui::tests`
- `pixi run codegen --force --check`

The exact full `cargo test -p re_selection_panel --features re_viewer_context/testing -- --test-threads=1` timed out in unrelated broader selection-panel tests; the targeted visualizer UI tests passed.

No code changed after those checks; only this status file and `/tmp` patch artifacts were written afterward.

## Patch scope

`/tmp/rerun_camera_pyramid_style_feature.patch` was generated from camera/pinhole feature paths only and excludes scalar LineStrips3D-only files.

One caveat: `crates/viewer/re_selection_panel/src/visualizer_ui.rs` already contained scalar UI changes in this dirty worktree, and the camera UI changes share that file. The patch therefore includes the mixed `visualizer_ui.rs` diff.

## VideoSfM integration prep

No VideoSfM files were edited.

Targeted search found existing VideoSfM frustum logging as custom `rr.LineStrips3D` wireframes in:

- `/home/zador/workspace/projects/VIDEOSFM/worktrees/videosfm-gp-callbacks/mpsfm/analysis/ra_trace/rerun.py`
- `/home/zador/workspace/projects/VIDEOSFM/worktrees/videosfm-gp-callbacks/mpsfm/utils/optim_recorder.py`

No obvious `rr.Pinhole` plus camera-transform logging integration point was found within the scoped search, so `/tmp/videosfm_camera_pyramid_style_integration.patch` was not created.

Proposed future integration: when VideoSfM has per-camera intrinsics and camera-to-world/world-to-camera transforms available at the Rerun logging site, log each camera as `rr.Transform3D` plus `rr.Pinhole(color=..., radius=...)` on per-camera entity paths. Until then, the existing `LineStrips3D` frustum logging remains the stock-compatible fallback.

## Limitations

- Camera pyramid style controls apply to Rerun `Pinhole` camera visualizer output. They do not automatically restyle arbitrary precomputed `LineStrips3D` frustum geometry.
- Older stock Rerun viewers do not understand the new `Pinhole.color` and `Pinhole.radius` fields. VideoSfM should keep a `LineStrips3D` fallback or gate use of the new API on a Rerun version that includes this feature.
- The generated patch is intended for this scalar-lines worktree context because of the mixed `visualizer_ui.rs` diff noted above.
