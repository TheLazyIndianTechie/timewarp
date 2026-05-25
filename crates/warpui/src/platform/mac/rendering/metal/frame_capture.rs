//! NOTE: The Metal-touching functions here are temporary skeletons committed in
//! the first step of the `objc2-metal` migration. Their full implementations
//! are restored in a follow-up commit. `convert_bgra_to_rgba` and its test are
//! pure and unchanged.

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTLDevice, MTLPixelFormat, MTLTexture};
use pathfinder_geometry::vector::Vector2F;
use warpui_core::platform::CapturedFrame;

#[cfg(test)]
#[path = "frame_capture_tests.rs"]
mod tests;

/// Captures a rendered frame from a Metal texture and returns the raw BGRA pixel data.
///
/// The data is returned in Metal's native BGRA format to avoid an expensive
/// pixel-format conversion on the render thread. Consumers that need RGBA
/// should call `CapturedFrame::ensure_rgba()`.
///
/// # Arguments
/// * `texture` - The Metal texture containing the rendered frame
/// * `size` - The dimensions of the texture (width, height)
///
/// # Returns
/// * `Some(CapturedFrame)` containing the RGBA pixel data if successful
/// * `None` if the texture dimensions are invalid
#[allow(unused_variables)]
pub fn capture_frame(
    texture: &ProtocolObject<dyn MTLTexture>,
    size: Vector2F,
) -> Option<CapturedFrame> {
    todo!()
}

#[cfg(test)]
pub(crate) fn convert_bgra_to_rgba(data: &mut [u8]) {
    for chunk in data.chunks_exact_mut(4) {
        chunk.swap(0, 2);
    }
}

/// Creates an off-screen Metal texture
///
/// This is a utility function for headless/off-screen rendering scenarios where
/// you need to render to a texture rather than a window drawable. Currently unused
/// but kept for future headless capture or visual regression testing support.
///
/// # Arguments
/// * `device` - The Metal device to create the texture on
/// * `width` - The width of the texture in pixels
/// * `height` - The height of the texture in pixels
/// * `pixel_format` - The pixel format (should match the drawable format)
///
/// # Returns
/// * A new Metal texture that can be rendered to and read back from
#[allow(dead_code)]
#[allow(unused_variables)]
pub fn create_capture_texture(
    device: &ProtocolObject<dyn MTLDevice>,
    width: usize,
    height: usize,
    pixel_format: MTLPixelFormat,
) -> Retained<ProtocolObject<dyn MTLTexture>> {
    todo!()
}
