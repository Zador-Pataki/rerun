# Rerun Backup Plan

This repository is separate from VideoSfM. Before removing any Rerun worktrees
or splitting changes, preserve dirty local state on fork branches.

## Worktrees

- `/home/zador/workspace/projects/install/rerun`
  - Original branch: `main`
  - Backup branch: `backup/videosfm-rerun-main-dirty-20260608`
  - Dirty theme: scalar line strips, colormap plumbing, spatial/camera UI,
    Python packaging hooks, and local helper/scratch files.
- `/home/zador/workspace/projects/install/rerun-scalar-lines-clean`
  - Original branch: `videosfm/scalar-lines3d-colormap`
  - Backup branch: `backup/videosfm-rerun-scalar-lines-dirty-20260608`
  - Dirty theme: camera pyramid/style controls, projection/orthographic
    blueprint schema/codegen, and native PNG sequence export.

## Cleanup Order

1. Push both backup branches to `fork`.
2. Verify each backup branch is reachable from `refs/remotes/fork/*`.
3. Only then split feature commits or remove worktrees.
4. Treat generated SDK/codegen files as source artifacts until a later focused
   cleanup proves they are reproducible and safe to drop.
