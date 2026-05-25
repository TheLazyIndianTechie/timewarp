//! Metal renderer internals.
//!
//! NOTE: This is a temporary skeleton committed in the first step of the
//! `objc2-metal` migration to lock the public API surface that `window.rs`
//! depends on. The full renderer implementation is restored in a follow-up
//! commit.

use objc2::runtime::ProtocolObject;
use objc2_metal::{MTLDevice, MTLPixelFormat};
use warpui_core::rendering;
use warpui_core::{fonts, Scene};

use crate::platform::mac::window::WindowState;

/// A structure that manages rendering scenes using a particular hardware
/// device.
pub struct Renderer;

impl Renderer {
    #[allow(unused_variables)]
    pub fn new(
        device: &ProtocolObject<dyn MTLDevice>,
        color_pixel_format: MTLPixelFormat,
        glyph_config: rendering::GlyphConfig,
    ) -> Self {
        todo!()
    }
}

impl super::super::Renderer for Renderer {
    #[allow(unused_variables)]
    fn render(&mut self, scene: &Scene, window: &WindowState, font_cache: &fonts::Cache) {
        todo!()
    }

    fn resize(&mut self, _window: &WindowState) {
        // TODO(alokedesai): Backport the optimization to only set the size of surface when a
        // window is resized to the Metal renderer.
    }
}
