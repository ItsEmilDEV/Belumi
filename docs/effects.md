# Belumi Screen Effects

Built-in, off by default (`effects = false` in `belumi.toml`). Options > Horizon Settings > Belumi Settings > Screen effects.

| Setting | Values |
|---|---|
| Vignette | off, light, medium, strong |
| Saturation | 70% to 140% |
| Contrast | 90% to 130% |
| Colour temperature | cold to hot |
| Sharpen | off, low, medium, high |
| Film grain | off, low, medium, high |

## How it works

The effects run inside the pass that already copies the world onto the screen (`world_post.wgsl`, entry points
`fs_copy_fx` and `fs_fxaa_fx`), so they add no render pass and no extra texture. The pass uses one 48 byte uniform and,
with sharpen on, four extra texture taps. The hand and the HUD are drawn afterwards and are not affected. Effects work
together with the resolution scale and FXAA; with everything off the old pipelines run unchanged.

## Cost

Measured on an RX 5700 at 1280x720 with every effect at its strongest, uncapped on the test terrain: 0.378 ms per frame
without, 0.420 ms with, so about 0.04 ms. The cost grows with the pixel count; expect roughly 0.4 ms at 4K. Frame time is
the whole frame, not a GPU timestamp of the pass alone.

## Not included

Bloom, depth of field, motion blur and SSAO need extra passes and targets; Horizon's shader packs
(`docs/api/shaders.md`) are the place for those.
