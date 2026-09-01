use gpui::{
    AnyElement, App, Bounds, Corners, Element, ElementId, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, Pixels, RenderImage, Style, Window,
};
use gpui_video_player::{gst, Video};
use gst::prelude::*;
use gstreamer_video::{VideoFormat, VideoInfo};
use image::{ImageBuffer, Rgba};
use smallvec::SmallVec;
use std::sync::Arc;
use yuv::{yuv_nv12_to_bgra, YuvBiPlanarImage, YuvConversionMode, YuvRange, YuvStandardMatrix};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VideoFit {
    Contain,
    Cover,
}

#[derive(Clone, Copy, Debug)]
struct Nv12Layout {
    width: u32,
    height: u32,
    y_offset: usize,
    uv_offset: usize,
    y_stride: usize,
    uv_stride: usize,
}

impl Nv12Layout {
    fn from_info(info: &VideoInfo, data_len: usize) -> Option<Self> {
        if info.format() != VideoFormat::Nv12 {
            return None;
        }

        let source_width = info.width();
        let source_height = info.height();
        let width = source_width & !1;
        let height = source_height & !1;
        if width == 0 || height == 0 {
            return None;
        }

        let y_stride = usize::try_from(*info.stride().first()?).ok()?;
        let uv_stride = usize::try_from(*info.stride().get(1)?).ok()?;
        let y_offset = *info.offset().first()?;
        let uv_offset = *info.offset().get(1)?;
        if y_stride < width as usize || uv_stride < width as usize {
            return None;
        }

        let y_end = y_offset
            .checked_add((height as usize - 1).checked_mul(y_stride)?)?
            .checked_add(width as usize)?;
        let uv_rows = height as usize / 2;
        let uv_end = uv_offset
            .checked_add((uv_rows - 1).checked_mul(uv_stride)?)?
            .checked_add(width as usize)?;
        if data_len < y_end.max(uv_end) {
            return None;
        }

        Some(Self {
            width,
            height,
            y_offset,
            uv_offset,
            y_stride,
            uv_stride,
        })
    }
}

fn video_info(video: &Video) -> Option<VideoInfo> {
    let pipeline = video.pipeline();
    let sink: gst::Element = pipeline.property("video-sink");
    let ghost_pad = sink
        .pads()
        .first()?
        .clone()
        .dynamic_cast::<gst::GhostPad>()
        .ok()?;
    let bin = ghost_pad.parent_element()?.downcast::<gst::Bin>().ok()?;
    let app_sink = bin.by_name("gpui_video")?;
    let caps = app_sink.pads().first()?.current_caps()?;
    VideoInfo::from_caps(&caps).ok()
}

fn current_layout(video: &Video, data_len: usize) -> Option<Nv12Layout> {
    Nv12Layout::from_info(&video_info(video)?, data_len)
}

pub(crate) fn frame_is_renderable(video: &Video, data_len: usize) -> bool {
    current_layout(video, data_len).is_some()
}

#[cfg(test)]
pub(crate) fn frame_converts(video: &Video, data: &[u8]) -> bool {
    current_layout(video, data.len())
        .and_then(|layout| StridedVideoElement::render_frame(data, layout))
        .is_some()
}

pub(crate) fn video(
    video: Video,
    id: impl Into<ElementId>,
    width: Pixels,
    height: Pixels,
    fit: VideoFit,
) -> AnyElement {
    StridedVideoElement {
        video,
        width,
        height,
        fit,
        id: id.into(),
    }
    .into_any_element()
}

struct StridedVideoElement {
    video: Video,
    width: Pixels,
    height: Pixels,
    fit: VideoFit,
    id: ElementId,
}

impl StridedVideoElement {
    fn fitted_bounds(
        bounds: Bounds<Pixels>,
        frame_width: u32,
        frame_height: u32,
        fit: VideoFit,
    ) -> Bounds<Pixels> {
        let container_width = f32::from(bounds.size.width);
        let container_height = f32::from(bounds.size.height);
        let width_scale = container_width / frame_width as f32;
        let height_scale = container_height / frame_height as f32;
        let scale = match fit {
            VideoFit::Contain => width_scale.min(height_scale),
            VideoFit::Cover => width_scale.max(height_scale),
        };
        let width = frame_width as f32 * scale;
        let height = frame_height as f32 * scale;
        Bounds::new(
            gpui::point(
                bounds.origin.x + gpui::px((container_width - width) * 0.5),
                bounds.origin.y + gpui::px((container_height - height) * 0.5),
            ),
            gpui::size(gpui::px(width), gpui::px(height)),
        )
    }

    fn render_frame(data: &[u8], layout: Nv12Layout) -> Option<Arc<RenderImage>> {
        let bgra = Self::convert_frame_to_bgra(data, layout)?;
        // GPUI's RenderImage byte contract is BGRA even though image::Frame
        // carries an Rgba type marker.
        let image = ImageBuffer::<Rgba<u8>, _>::from_raw(layout.width, layout.height, bgra)?;
        let frames: SmallVec<[image::Frame; 1]> = SmallVec::from_elem(image::Frame::new(image), 1);
        Some(Arc::new(RenderImage::new(frames)))
    }

    fn convert_frame_to_bgra(data: &[u8], layout: Nv12Layout) -> Option<Vec<u8>> {
        let image = YuvBiPlanarImage {
            y_plane: data.get(layout.y_offset..)?,
            y_stride: u32::try_from(layout.y_stride).ok()?,
            uv_plane: data.get(layout.uv_offset..)?,
            uv_stride: u32::try_from(layout.uv_stride).ok()?,
            width: layout.width,
            height: layout.height,
        };
        let mut bgra = vec![0; layout.width as usize * layout.height as usize * 4];
        let output_stride = layout.width * 4;
        for (range, matrix) in [
            (YuvRange::Limited, YuvStandardMatrix::Bt709),
            (YuvRange::Full, YuvStandardMatrix::Bt709),
            (YuvRange::Limited, YuvStandardMatrix::Bt601),
        ] {
            if yuv_nv12_to_bgra(
                &image,
                &mut bgra,
                output_stride,
                range,
                matrix,
                YuvConversionMode::Balanced,
            )
            .is_ok()
            {
                return Some(bgra);
            }
        }
        None
    }
}

impl Element for StridedVideoElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = self.width.into();
        style.size.height = self.height.into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout_state: &mut Self::RequestLayoutState,
        window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        if (!self.video.eos() && !self.video.paused()) || self.video.take_frame_ready() {
            window.request_animation_frame();
        }
    }

    fn paint(
        &mut self,
        _global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout_state: &mut Self::RequestLayoutState,
        _prepaint_state: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some((data, _, _)) = self.video.current_frame_data() else {
            return;
        };
        let Some(layout) = current_layout(&self.video, data.len()) else {
            return;
        };
        let Some(render_image) = Self::render_frame(&data, layout) else {
            return;
        };
        let previous: gpui::Entity<Option<Arc<RenderImage>>> = window.use_state(cx, |_, _| None);
        let old = previous.update(cx, |state, _| state.replace(Arc::clone(&render_image)));
        let fitted = Self::fitted_bounds(bounds, layout.width, layout.height, self.fit);
        let _ = window.paint_image(
            fitted,
            Corners::default(),
            Arc::clone(&render_image),
            0,
            false,
        );
        if let Some(old) = old {
            cx.drop_image(old, Some(window));
        }
    }
}

impl IntoElement for StridedVideoElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{Nv12Layout, StridedVideoElement, VideoFit};
    use gpui::{point, px, size, Bounds, Pixels};

    fn assert_px(actual: Pixels, expected: f32) {
        assert!((f32::from(actual) - expected).abs() < 0.01);
    }

    #[test]
    fn framing_modes_fit_or_fill_the_tile() {
        let bounds = Bounds::new(point(px(0.0), px(0.0)), size(px(100.0), px(100.0)));
        let contained = StridedVideoElement::fitted_bounds(bounds, 200, 100, VideoFit::Contain);
        assert_px(contained.origin.x, 0.0);
        assert_px(contained.origin.y, 25.0);
        assert_px(contained.size.width, 100.0);
        assert_px(contained.size.height, 50.0);

        let covered = StridedVideoElement::fitted_bounds(bounds, 200, 100, VideoFit::Cover);
        assert_px(covered.origin.x, -50.0);
        assert_px(covered.origin.y, 0.0);
        assert_px(covered.size.width, 200.0);
        assert_px(covered.size.height, 100.0);
    }

    #[test]
    fn tight_nv12_matches_gpui_bgra_contract() {
        // Limited-range BT.709 NV12 approximating solid red.
        let data = [63, 63, 63, 63, 102, 240];
        let render_image = StridedVideoElement::render_frame(
            &data,
            Nv12Layout {
                width: 2,
                height: 2,
                y_stride: 2,
                uv_stride: 2,
                y_offset: 0,
                uv_offset: 4,
            },
        )
        .expect("convert NV12 frame");
        let bgra = render_image.as_bytes(0).expect("render image bytes");

        assert!(
            bgra[2] > bgra[0],
            "RenderImage red and blue channels are swapped: {bgra:?}"
        );
        assert_eq!(bgra[3], 255);
    }
}
