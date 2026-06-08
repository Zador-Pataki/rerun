# VideoSfM scalar LineStrips3D status

## Implemented

This Rerun checkout has a prototype generic scalar-coloring path for `LineStrips3D`:

- Python/Rust `LineStrips3D` accepts per-strip `scalar_values`, a `scalar_range`, and a `colormap`.
- The 3D line visualizer colors each strip in the viewer from scalar values instead of relying on baked RGB colors.
- `RedToGreen` / `"red_green"` is available as a colormap.
- Scalar line entities no longer auto-spawn a parallel time-series view just because they carry `Scalar`.
- The Selection panel promotes entity-level scalar controls for scalar-colored line groups.

User workflow:

1. Open a recording containing scalar-colored line strips.
2. Select/click the line-strip group entity, e.g. `world/lc_edges/force`.
3. In the right Selection panel, use:
   `Visualizers -> Lines3D -> Scalar coloring -> Saturation max`
4. Drag or edit `Saturation max` to change the upper bound of the scalar color range.

The UI writes a blueprint override for `rerun.components.ValueRange` on the selected data-result override path. The `.rrd` recording is unchanged, and recoloring happens viewer-side.

## VideoSfM integration contract

Current VideoSfM rho-pull logging is in:

`/home/zador/workspace/projects/VIDEOSFM/worktrees/videosfm-gp-callbacks/mpsfm/analysis/gp_trace/visualization/rerun.py`

The current path logs baked colors at `world/lc_edges/force`:

```python
rr.LineStrips3D(strips, colors=colors, radii=lc_edges.radius)
```

The minimal future change is to log scalar values for the same strips:

```python
rr.LineStrips3D(
    strips,
    scalar_values=force_scores_for_logged_strips,
    scalar_range=[0.0, lc_edges.force_reference],
    colormap="red_green",
    radii=lc_edges.radius,
)
```

`force_scores_for_logged_strips` must stay aligned with the filtered `strips` list. In practice, `_lc_edge_force_primitives` should either return `(strips, scores)` for the edges that have frame centers, or VideoSfM should build `strips` and scalar scores together in the rho-pull branch.

Stock Rerun 0.22.1 does not accept `scalar_values`, `scalar_range`, or `colormap` on `LineStrips3D`. VideoSfM therefore needs feature detection/fallback if it must support stock Rerun:

- try constructing/logging with scalar arguments when supported;
- otherwise keep the current baked `colors` path.

## Synthetic proof

Generated:

- `/tmp/rerun-videosfm-lc-force-scalar-lines.rrd`
- `/tmp/rerun-videosfm-lc-force-scalar-lines-range-100.rbl`

The `.rrd` logs `world/lc_edges/force` with scalar values `[0, 25, 50, 100]`, `scalar_range=[0, 50]`, and `colormap="red_green"`.

Screenshots:

- `/tmp/rerun-videosfm-lc-force-scalar-lines-range-50.png`
- `/tmp/rerun-videosfm-lc-force-scalar-lines-range-100.png`

The second screenshot uses the same `.rrd` plus a blueprint override to set `scalar_range=[0, 100]`; it visibly recolors the lines without regenerating the recording.

## Verification

Passed in this checkout:

```bash
cargo build --package rerun-cli --release
cargo check -p re_selection_panel
cargo check -p re_view_spatial -p re_view_time_series
cargo test -p re_selection_panel --features re_viewer_context/testing
cargo test -p re_types --test types line_strips3d
PYTEST_DISABLE_PLUGIN_AUTOLOAD=1 python -m pytest rerun_py/tests/unit/test_line_strips3d.py
```

Known test/runtime caveats:

- Plain `cargo test -p re_selection_panel` needs the `re_viewer_context/testing` feature for an existing test dependency.
- Screenshot mode saves screenshots, then panics on shutdown with `Failed to take store hub from the Viewer`.
- A previous full `re_view_time_series` test run failed because a Git LFS PNG snapshot was not materialized in this checkout.

## Patch hygiene inventory

Feature patch:

- `/tmp/rerun_scalar_lines_feature.patch`

Files definitely required for scalar-colored `LineStrips3D` and group saturation UI:

- `crates/store/re_types/definitions/rerun/archetypes/line_strips3d.fbs`
- `crates/store/re_types/definitions/rerun/components/colormap.fbs`
- `crates/store/re_types/src/archetypes/line_strips3d.rs`
- `crates/store/re_types/src/components/colormap.rs`
- `crates/store/re_types/src/components/colormap_ext.rs`
- `crates/store/re_types/tests/types/line_strips3d.rs`
- `crates/viewer/re_renderer/shader/colormap.wgsl`
- `crates/viewer/re_renderer/src/colormap.rs`
- `crates/viewer/re_renderer/src/lib.rs`
- `crates/viewer/re_viewer_context/src/gpu_bridge/colormap.rs`
- `crates/viewer/re_view_spatial/src/visualizers/lines3d.rs`
- `crates/viewer/re_view_time_series/src/view_class.rs`
- `crates/viewer/re_selection_panel/src/visualizer_ui.rs`
- `rerun_py/rerun_sdk/rerun/archetypes/line_strips3d.py`
- `rerun_py/rerun_sdk/rerun/components/colormap.py`
- `rerun_py/tests/unit/test_line_strips3d.py`

Probably unrelated / pre-existing dirty work in this checkout:

- `Cargo.lock`
- `crates/viewer/re_renderer/src/view_builder.rs`
- `crates/viewer/re_view_map/src/map_view.rs`
- `crates/viewer/re_view_spatial/src/eye.rs`
- `crates/viewer/re_view_spatial/src/lib.rs`
- `crates/viewer/re_view_spatial/src/pinhole.rs`
- `crates/viewer/re_view_spatial/src/ui.rs`
- `crates/viewer/re_view_spatial/src/ui_2d.rs`
- `crates/viewer/re_view_spatial/src/ui_3d.rs`
- `crates/viewer/re_view_spatial/src/visualizers/cameras.rs`
- `crates/viewer/re_viewer/Cargo.toml`
- `crates/viewer/re_viewer/src/lib.rs`
- `crates/viewer/re_viewer/src/ui/rerun_menu.rs`
- `crates/viewer/re_viewer_context/src/global_context/app_options.rs`
- `crates/viewer/re_viewer_context/src/gpu_bridge/mod.rs`
- `crates/viewer/re_viewport/src/viewport_ui.rs`
- `examples/python/nuscenes_dataset/nuscenes_dataset/__main__.py`
- `pixi.lock`
- `rerun_notebook/package-lock.json`
- `rerun_py/Cargo.toml`
- `rerun_py/pyproject.toml`
- `rerun_py/rerun_sdk/rerun/__init__.py`
- `rerun_py/src/lib.rs`
- `rerun_py/src/python_bridge.rs`
- `rerun_py/src/viewer.rs`
- untracked local files such as `build_rerun.sh`, `my_test.py`, `rerun_sdk/`, and `videosfm_rerun_fork_prompt.md`

Uncertain / needs review before a clean branch:

- `crates/viewer/re_view_spatial/src/ui.rs`
- `crates/viewer/re_view_spatial/src/ui_2d.rs`
- `crates/viewer/re_view_spatial/src/ui_3d.rs`
- `crates/viewer/re_view_spatial/src/visualizers/cameras.rs`

These were already dirty, but `cargo fmt --package re_view_spatial` may have touched formatting while working on the scalar-line feature. Review before discarding or keeping.

## Filtering and metadata assessment

`Hide outside scalar range` is not implemented.

Draw-time filtering itself would be local to `Lines3DVisualizer`: skip strips whose scalar is outside `ValueRange`. The blocker is the user-facing toggle. A clean generic toggle needs a new component or blueprint property that can be overridden per selected entity/group. Reusing existing `Visible`, `SeriesVisible`, or `ShowLabels` would be semantically wrong or conflict with existing visibility behavior. Adding a new component means schema/codegen/Python/Rust/UI changes similar in shape to the scalar range work, so it is not a small add-on.

Per-strip hover metadata is partially available: `Lines3DVisualizer` already assigns picking instance ids by strip index, and scalar values are logged as per-instance scalar components. However, stable user-defined strip ids are not implemented. A clean version would add a per-strip id/label component or document that strip index is the stable identity within a logged batch.

## Current readiness

Ready for VideoSfM integration behind feature detection/fallback. The main remaining work is packaging this into a clean Rerun branch, regenerating code from schema once local codegen is healthy, and adding a stock-Rerun fallback path in VideoSfM.
