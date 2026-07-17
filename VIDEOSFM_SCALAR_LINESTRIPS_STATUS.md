# VideoSfM Scalar LineStrips3D Status

## Implemented in this branch

- `rr.LineStrips3D` accepts optional per-strip `scalar_values`, `scalar_range`, and `colormap`.
- The spatial `Lines3D` visualizer maps scalar values to colors in the viewer. Values outside the range are clamped.
- The selection panel exposes group-level scalar coloring controls through blueprint overrides, so the recording is not regenerated.
- Added a `RedToGreen` / `red_green` colormap for low-to-high score visualization.

## User workflow

Open a recording with scalar-colored line strips, then:

1. Select the entity, for example `world/lc_edges/force`.
2. Open `Selection panel -> Visualizers -> Lines3D -> Scalar coloring`.
3. Adjust `Saturation max` to change the upper end of the scalar color range for the whole selected entity/group.
4. Optional numeric `Min`/`Max` fields and colormap selection are in the same section.

The UI writes a selected-entity blueprint override for `ValueRange`/`Colormap`; the `.rrd` data stays unchanged.

## VideoSfM integration contract

For LC edge rho-pull mode, VideoSfM can log:

```python
rr.LineStrips3D(
    strips,
    radii=lc_edges.radius,
    scalar_values=force_scores_for_strips,
    scalar_range=[0.0, lc_edges.force_reference],
    colormap="red_green",
)
```

Use the entity path `world/lc_edges/force` as today. The score order must match the strip order.

## Stock Rerun fallback

Stock Rerun 0.22.1 does not accept `scalar_values`, `scalar_range`, or `colormap` on `LineStrips3D`. VideoSfM should feature-detect support and fall back to baked RGB colors when unavailable.

## Verification

Clean worktree:

- Path: `/home/zador/workspace/projects/install/rerun-scalar-lines-clean`
- Branch: `videosfm/scalar-lines3d-colormap`
- Source patch applied cleanly from `/tmp/rerun_scalar_lines_feature.patch`.

Commands run:

```bash
cargo build --package rerun-cli --release
cargo check -p re_selection_panel
cargo check -p re_view_spatial -p re_view_time_series
cargo test -p re_selection_panel --features re_viewer_context/testing
cargo test -p re_types --test types line_strips3d
PYTEST_DISABLE_PLUGIN_AUTOLOAD=1 python -m pytest rerun_py/tests/unit/test_line_strips3d.py
```

Results:

- `cargo build --package rerun-cli --release`: passed after copying ignored generated web viewer artifacts (`web_viewer/re_viewer.js`, `web_viewer/re_viewer_bg.wasm`) from the original checkout. The first attempt failed because a fresh worktree lacks those generated files.
- `cargo check -p re_selection_panel`: passed.
- `cargo check -p re_view_spatial -p re_view_time_series`: passed.
- `cargo test -p re_selection_panel --features re_viewer_context/testing`: passed, 3 tests.
- `cargo test -p re_types --test types line_strips3d`: passed, 1 filtered test.
- `PYTEST_DISABLE_PLUGIN_AUTOLOAD=1 python -m pytest rerun_py/tests/unit/test_line_strips3d.py`: passed, 6 tests. `rerun_py` was built locally with Cargo and the resulting extension was copied into the clean worktree package; the VideoSfM venv was not modified.

Synthetic proof target:

- Recording: `/tmp/rerun-videosfm-lc-force-scalar-lines.rrd`
- Range-100 blueprint: `/tmp/rerun-videosfm-lc-force-scalar-lines-range-100.rbl`
- Screenshots: `/tmp/rerun-videosfm-lc-force-scalar-lines-clean-range-50.png`, `/tmp/rerun-videosfm-lc-force-scalar-lines-clean-range-100.png`
- Screenshot mode saved images, then hit the pre-existing screenshotter shutdown panic: `Failed to take store hub from the Viewer`.

## Limitations

- Filtering/hiding outside the scalar range is not implemented. A clean version needs a new schema/blueprint property; overloading existing visibility components would conflict with normal entity visibility semantics.
- Stable user-provided per-strip IDs are not implemented. Picking currently has strip instance indices, but stable IDs require a new component/convention.
- The first UI is path/entity-level, not a global scalar-color manager.
