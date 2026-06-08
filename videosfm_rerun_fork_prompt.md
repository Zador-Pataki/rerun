You are working in a separate tmux Codex session for the Rerun source checkout at:

  /home/zador/workspace/projects/install/rerun

Keep this isolated from the main VideoSfM session.

Goal
----
Investigate and, if feasible, implement the custom Rerun viewer/Python SDK feature needed by VideoSfM GP trace visualization:

  scalar-colored 3D line strips with in-viewer controls for scalar range / saturation threshold.

Concrete VideoSfM use case
--------------------------
VideoSfM logs GP loop-closure edges as `LineStrips3D`. For each LC edge and optimizer iteration, VideoSfM can compute:

  lc_edge_force = sum(rho1 * sqrt(rho0)) over the residuals assigned to that LC edge

Today VideoSfM bakes colors in Python before logging:

  score 0 -> red
  score >= force_reference -> green
  interpolation in between

The user wants the `force_reference` / scalar color range adjustable inside the Rerun viewer, without regenerating the recording.

Important constraints
---------------------
- Do not edit `/home/zador/workspace/projects/VIDEOSFM/worktrees/videosfm-gp-callbacks` unless explicitly asked. Treat it as a read-only consumer/test fixture.
- Do not install or uninstall packages in the shared VideoSfM venv unless explicitly approved.
- Prefer `PYTHONPATH` / `PATH` overlay when testing custom Rerun with VideoSfM.
- The VideoSfM worktree is dirty; do not reset, checkout, clean, or format it.
- The Rerun checkout is dirty too. Inspect current changes before editing and preserve existing work.
- If you use git status in Rerun and hit nbstripout filter issues, use:

  git -c filter.nbstripout.clean=cat -c filter.nbstripout.smudge=cat -c filter.nbstripout.required=false status --short --branch

Useful paths
------------
Rerun source:

  /home/zador/workspace/projects/install/rerun

VideoSfM worktree:

  /home/zador/workspace/projects/VIDEOSFM/worktrees/videosfm-gp-callbacks

VideoSfM venv:

  /home/zador/workspace/projects/VIDEOSFM/videosfm_venv_colmap4-glomap-refactor

Current VideoSfM integration points
-----------------------------------
Read these first:

  /home/zador/workspace/projects/VIDEOSFM/worktrees/videosfm-gp-callbacks/mpsfm/analysis/gp_trace/visualization/rerun.py
  /home/zador/workspace/projects/VIDEOSFM/worktrees/videosfm-gp-callbacks/mpsfm/analysis/gp_trace/commands/rerun_cli.py
  /home/zador/workspace/projects/VIDEOSFM/worktrees/videosfm-gp-callbacks/mpsfm/analysis/gp_trace/static_index.py

Important current functions/classes:

  GlobalPositioningTraceRerun
  GlobalPositioningTraceRerunLCEdgeOptions
  _log_lc_edges_or_clear
  _lc_edge_force_for_iteration
  _lc_edge_rho_pull_from_loss_rho_values
  _lc_edge_force_primitives

Current Rerun 0.22.1 Python API finding
---------------------------------------
`LineStrips3D` currently accepts only:

  strips, radii, colors, labels, show_labels, class_ids

There is no first-class scalar field or viewer-side scalar colormap control for line strips.

Python can log metadata using `AnyValues` or custom components, but the 3D line visualizer does not consume arbitrary scalar columns for coloring/filtering/picking details.

Preferred design direction
--------------------------
Implement a generic Rerun feature, not a VideoSfM-specific feature.

Desired API shape, exact naming flexible:

  rr.LineStrips3D(
      strips,
      radii=0.018,
      scalar_values=force_scores,
      scalar_name="lc_edge_force",
      scalar_range=(0.0, 50.0),
      colormap="red_green",
      ...
  )

or an equivalent component-based API if that better matches Rerun architecture.

Viewer behavior wanted:

- Per-line-strip scalar values.
- Viewer-side scalar color mapping for `LineStrips3D`.
- UI control for min/max or saturation/reference threshold.
- Ideally filtering/hiding by scalar range.
- Picking/hover details should expose scalar values and stable per-strip ids if possible.
- It should work across timeline updates.

Please do this in phases
------------------------
1. Inspect existing local Rerun changes and summarize what is already dirty.
2. Locate the line-strip schema/type definitions and the Rust 3D line visualizer.
3. Decide the smallest generic feature that gets an in-viewer scalar color slider for line strips.
4. Implement a minimal prototype if feasible.
5. Build/test Rerun locally.
6. Verify from Python with a tiny synthetic recording before touching VideoSfM integration.
7. Then test VideoSfM only through overlay environment variables.

Overlay environment for VideoSfM tests
--------------------------------------
Use this, do not install into the venv:

  source /home/zador/workspace/projects/VIDEOSFM/videosfm_venv_colmap4-glomap-refactor/bin/activate
  export RERUN_SRC=/home/zador/workspace/projects/install/rerun
  export VIDEOSFM=/home/zador/workspace/projects/VIDEOSFM/worktrees/videosfm-gp-callbacks
  export PYTHONPATH="$RERUN_SRC/rerun_py/rerun_sdk:$RERUN_SRC/rerun_py:$VIDEOSFM:${PYTHONPATH:-}"
  export PATH="$RERUN_SRC/target/release:$PATH"

Verify import source:

  python - <<'PY'
  import rerun as rr, mpsfm
  print("rerun", rr.__version__, rr.__file__)
  print("mpsfm", mpsfm.__file__)
  PY

Relevant VideoSfM tests:

  cd /home/zador/workspace/projects/VIDEOSFM/worktrees/videosfm-gp-callbacks
  python -m pytest tests/analysis/test_gp_trace_rerun_import_boundary.py tests/analysis/test_gp_trace_rerun_cli.py tests/mapper/test_gp_trace_rerun.py

Prefer save_rrd tests before spawning viewer:

  python scripts/replay_gp_trace_rerun.py /path/to/run/gp_trace \
      --mode save_rrd \
      --save-path /tmp/videosfm-rerun-custom.rrd \
      --max-points 1000

Open with custom viewer:

  /home/zador/workspace/projects/install/rerun/target/release/rerun /tmp/videosfm-rerun-custom.rrd

Reporting expectation
---------------------
Be explicit about whether the feature is:

- possible as a small patch,
- possible but invasive,
- blocked by Rerun architecture,
- or better implemented as a separate VideoSfM live-controller.

Do not claim success unless a synthetic Python recording proves that a line-strip scalar can be recolored from the viewer side.
