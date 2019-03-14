# TODO

## Use template for bad words response.

## Graphics

Simplify main loop by abstracting the rendering for each frame.
Push command modifications that are included in each frame (lock-free queue?).

Command modifications:
* Create a new image at coordinate.

For each frame, animate and apply physics to images (pushed through uniforms).

Note: Make sure data layout is compatible with what is expected by the shader.

Bind uniform buffers and flush the corresponding range to the GPU per-frame:
https://github.com/SaschaWillems/Vulkan/blob/master/examples/dynamicuniformbuffer/dynamicuniformbuffer.cpp#L509

## General

## Playback

* Allow removing songs.

# ChangeLog

* Words and commands now have their own frontends.
* Implemented counters.
* Song requests through Spotify.