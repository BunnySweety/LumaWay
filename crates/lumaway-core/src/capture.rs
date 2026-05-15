use std::os::fd::{AsRawFd, OwnedFd};
use std::time::{Duration, Instant};

use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;

use crate::{CoreError, Result};

#[derive(Debug, Clone, Copy)]
pub struct CaptureStats {
    pub frames: u64,
    pub duration: Duration,
    pub fps: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbAverage {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SamplePoint {
    pub x: f64,
    pub y: f64,
}

impl SamplePoint {
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            x: x.clamp(0.0, 1.0),
            y: y.clamp(0.0, 1.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SampleRegion {
    pub center: SamplePoint,
    pub width: f64,
    pub height: f64,
}

impl SampleRegion {
    pub fn new(center: SamplePoint, width: f64, height: f64) -> Self {
        Self {
            center,
            width: width.clamp(0.001, 1.0),
            height: height.clamp(0.001, 1.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DetectedSampleCrop {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
}

impl DetectedSampleCrop {
    pub const NONE: Self = Self {
        left: 0.0,
        right: 0.0,
        top: 0.0,
        bottom: 0.0,
    };
}

pub struct GStreamerTestCapture {
    pipeline: gst::Pipeline,
    appsink: gst_app::AppSink,
    profile: CaptureProfile,
    backend: CaptureBackend,
    _pipewire_fd: Option<OwnedFd>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureBackend {
    Cpu,
    Gl,
}

#[derive(Debug, Clone)]
pub struct SampleBenchFrame {
    pub capture_duration: Duration,
    pub grids: Vec<SampleGridTiming>,
}

#[derive(Debug, Clone, Copy)]
pub struct SampleGridTiming {
    pub profile: CaptureProfile,
    pub duration: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureProfile {
    pub width: i32,
    pub height: i32,
    pub fps: i32,
}

impl CaptureProfile {
    pub fn new(width: i32, height: i32, fps: i32) -> Result<Self> {
        if width <= 0 || height <= 0 || fps <= 0 {
            return Err(CoreError::GStreamer(format!(
                "invalid capture profile: {width}x{height}@{fps}"
            )));
        }

        Ok(Self { width, height, fps })
    }

    pub fn point_sample_radius(self) -> (usize, usize) {
        point_sample_radius(self.width as usize, self.height as usize)
    }
}

impl GStreamerTestCapture {
    pub fn new(width: i32, height: i32, fps: i32) -> Result<Self> {
        gst::init().map_err(|err| CoreError::GStreamer(err.to_string()))?;
        let profile = CaptureProfile::new(width, height, fps)?;

        let pipeline = build_appsink_pipeline(CaptureSource::VideoTest { profile })?;

        Ok(Self {
            pipeline: pipeline.pipeline,
            appsink: pipeline.appsink,
            profile,
            backend: CaptureBackend::Cpu,
            _pipewire_fd: None,
        })
    }

    pub fn from_pipewire_node(
        node_id: u32,
        pipewire_fd: OwnedFd,
        width: i32,
        height: i32,
        fps: i32,
    ) -> Result<Self> {
        Self::from_pipewire_node_with_backend(
            node_id,
            pipewire_fd,
            width,
            height,
            fps,
            CaptureBackend::Cpu,
        )
    }

    pub fn from_pipewire_node_with_backend(
        node_id: u32,
        pipewire_fd: OwnedFd,
        width: i32,
        height: i32,
        fps: i32,
        backend: CaptureBackend,
    ) -> Result<Self> {
        gst::init().map_err(|err| CoreError::GStreamer(err.to_string()))?;
        let profile = CaptureProfile::new(width, height, fps)?;

        let pipeline = build_appsink_pipeline(CaptureSource::PipeWire {
            node_id,
            fd: pipewire_fd.as_raw_fd(),
            backend,
        })?;

        Ok(Self {
            pipeline: pipeline.pipeline,
            appsink: pipeline.appsink,
            profile,
            backend,
            _pipewire_fd: Some(pipewire_fd),
        })
    }

    pub fn backend(&self) -> CaptureBackend {
        self.backend
    }

    pub fn run_for(&self, duration: Duration) -> Result<CaptureStats> {
        self.pipeline
            .set_state(gst::State::Playing)
            .map_err(|err| {
                CoreError::GStreamer(format!(
                    "failed to start pipeline: {err:?}; {}",
                    self.bus_error_detail()
                ))
            })?;

        let start = Instant::now();
        let deadline = start + duration;
        let mut frames = 0u64;

        while Instant::now() < deadline {
            let sample = self
                .appsink
                .try_pull_sample(gst::ClockTime::from_seconds(5))
                .ok_or(CoreError::CaptureTimeout)?;

            if sample.buffer().is_some() {
                frames += 1;
            }
        }

        self.pipeline
            .set_state(gst::State::Null)
            .map_err(|err| CoreError::GStreamer(format!("failed to stop pipeline: {err:?}")))?;

        let elapsed = start.elapsed();
        let fps = frames as f64 / elapsed.as_secs_f64();
        Ok(CaptureStats {
            frames,
            duration: elapsed,
            fps,
        })
    }

    pub fn start(&self) -> Result<()> {
        self.pipeline
            .set_state(gst::State::Playing)
            .map_err(|err| {
                CoreError::GStreamer(format!(
                    "failed to start pipeline: {err:?}; {}",
                    self.bus_error_detail()
                ))
            })?;
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        self.pipeline
            .set_state(gst::State::Null)
            .map_err(|err| CoreError::GStreamer(format!("failed to stop pipeline: {err:?}")))?;
        Ok(())
    }

    pub fn pull_average_color(&self, timeout: Duration) -> Result<RgbAverage> {
        let timeout =
            gst::ClockTime::from_nseconds(timeout.as_nanos().min(u64::MAX as u128) as u64);
        let sample = self
            .appsink
            .try_pull_sample(timeout)
            .ok_or(CoreError::CaptureTimeout)?;
        average_sample_rgb(&sample)
    }

    pub fn pull_horizontal_averages(
        &self,
        bands: usize,
        timeout: Duration,
    ) -> Result<Vec<RgbAverage>> {
        self.pull_horizontal_averages_with_profile(bands, timeout, self.profile)
    }

    pub fn pull_horizontal_averages_with_profile(
        &self,
        bands: usize,
        timeout: Duration,
        profile: CaptureProfile,
    ) -> Result<Vec<RgbAverage>> {
        let timeout =
            gst::ClockTime::from_nseconds(timeout.as_nanos().min(u64::MAX as u128) as u64);
        let sample = self
            .appsink
            .try_pull_sample(timeout)
            .ok_or(CoreError::CaptureTimeout)?;
        average_sample_horizontal_bands(&sample, bands, profile)
    }

    pub fn pull_point_averages(
        &self,
        points: &[SamplePoint],
        timeout: Duration,
    ) -> Result<Vec<RgbAverage>> {
        let timeout =
            gst::ClockTime::from_nseconds(timeout.as_nanos().min(u64::MAX as u128) as u64);
        let sample = self
            .appsink
            .try_pull_sample(timeout)
            .ok_or(CoreError::CaptureTimeout)?;
        average_sample_points(&sample, points, self.profile)
    }

    pub fn pull_region_averages(
        &self,
        regions: &[SampleRegion],
        timeout: Duration,
    ) -> Result<Vec<RgbAverage>> {
        let timeout =
            gst::ClockTime::from_nseconds(timeout.as_nanos().min(u64::MAX as u128) as u64);
        let sample = self
            .appsink
            .try_pull_sample(timeout)
            .ok_or(CoreError::CaptureTimeout)?;
        average_sample_regions(&sample, regions, self.profile)
    }

    pub fn pull_detected_black_bars(
        &self,
        threshold: u8,
        timeout: Duration,
    ) -> Result<DetectedSampleCrop> {
        let timeout =
            gst::ClockTime::from_nseconds(timeout.as_nanos().min(u64::MAX as u128) as u64);
        let sample = self
            .appsink
            .try_pull_sample(timeout)
            .ok_or(CoreError::CaptureTimeout)?;
        detect_sample_black_bars(&sample, threshold, self.profile)
    }

    pub fn benchmark_horizontal_averages_profiles(
        &self,
        bands: usize,
        timeout: Duration,
        profiles: &[CaptureProfile],
    ) -> Result<SampleBenchFrame> {
        let timeout =
            gst::ClockTime::from_nseconds(timeout.as_nanos().min(u64::MAX as u128) as u64);
        let capture_started = Instant::now();
        let sample = self
            .appsink
            .try_pull_sample(timeout)
            .ok_or(CoreError::CaptureTimeout)?;
        let capture_duration = capture_started.elapsed();

        let mut grids = Vec::with_capacity(profiles.len());
        for profile in profiles {
            let started = Instant::now();
            let _averages = average_sample_horizontal_bands(&sample, bands, *profile)?;
            grids.push(SampleGridTiming {
                profile: *profile,
                duration: started.elapsed(),
            });
        }

        Ok(SampleBenchFrame {
            capture_duration,
            grids,
        })
    }
}

impl GStreamerTestCapture {
    fn bus_error_detail(&self) -> String {
        let Some(bus) = self.pipeline.bus() else {
            return "pipeline has no bus".into();
        };

        match bus.timed_pop_filtered(
            gst::ClockTime::from_mseconds(250),
            &[gst::MessageType::Error, gst::MessageType::Warning],
        ) {
            Some(message) => match message.view() {
                gst::MessageView::Error(error) => {
                    format!(
                        "bus error from {:?}: {}; debug={:?}",
                        error.src().map(|src| src.path_string()),
                        error.error(),
                        error.debug()
                    )
                }
                gst::MessageView::Warning(warning) => {
                    format!(
                        "bus warning from {:?}: {}; debug={:?}",
                        warning.src().map(|src| src.path_string()),
                        warning.error(),
                        warning.debug()
                    )
                }
                _ => "no error detail available".into(),
            },
            None => "no bus error detail available".into(),
        }
    }
}

struct AppsinkPipeline {
    pipeline: gst::Pipeline,
    appsink: gst_app::AppSink,
}

enum CaptureSource {
    VideoTest {
        profile: CaptureProfile,
    },
    PipeWire {
        node_id: u32,
        fd: i32,
        backend: CaptureBackend,
    },
}

fn build_appsink_pipeline(source: CaptureSource) -> Result<AppsinkPipeline> {
    let pipeline = gst::Pipeline::new();

    match source {
        CaptureSource::VideoTest { profile } => {
            let source = make_element("videotestsrc")?;
            let convert = make_element("videoconvert")?;
            let scale = make_element("videoscale")?;
            let capsfilter = make_element("capsfilter")?;
            let appsink = make_element("appsink")?
                .dynamic_cast::<gst_app::AppSink>()
                .map_err(|_| CoreError::GStreamer("failed to cast appsink".into()))?;

            source.set_property("is-live", true);
            source.set_property_from_str("pattern", "smpte");

            let caps = rgb_caps(profile);
            capsfilter.set_property("caps", &caps);

            appsink.set_property("emit-signals", false);
            appsink.set_property("sync", false);
            appsink.set_property("max-buffers", 1u32);
            appsink.set_property("drop", true);

            pipeline
                .add_many([&source, &convert, &scale, &capsfilter, appsink.upcast_ref()])
                .map_err(|err| CoreError::GStreamer(err.to_string()))?;

            gst::Element::link_many([&source, &convert, &scale, &capsfilter, appsink.upcast_ref()])
                .map_err(|err| CoreError::GStreamer(err.to_string()))?;

            Ok(AppsinkPipeline { pipeline, appsink })
        }
        CaptureSource::PipeWire {
            node_id,
            fd,
            backend,
        } => {
            let source = make_element("pipewiresrc")?;
            let appsink = make_element("appsink")?
                .dynamic_cast::<gst_app::AppSink>()
                .map_err(|_| CoreError::GStreamer("failed to cast appsink".into()))?;

            source.set_property("fd", fd);
            source.set_property("path", node_id.to_string());
            source.set_property("do-timestamp", true);
            source.set_property("keepalive-time", 100i32);
            source.set_property("resend-last", true);

            appsink.set_property("emit-signals", false);
            appsink.set_property("max-buffers", 1u32);
            appsink.set_property("drop", true);
            appsink.set_property("sync", false);

            match backend {
                CaptureBackend::Cpu => {
                    let convert = make_element("videoconvert")?;
                    pipeline
                        .add_many([&source, &convert, appsink.upcast_ref()])
                        .map_err(|err| CoreError::GStreamer(err.to_string()))?;

                    gst::Element::link_many([&source, &convert, appsink.upcast_ref()])
                        .map_err(|err| CoreError::GStreamer(err.to_string()))?;
                }
                CaptureBackend::Gl => {
                    let upload = make_element("glupload")?;
                    let gl_convert = make_element("glcolorconvert")?;
                    let download = make_element("gldownload")?;
                    let convert = make_element("videoconvert")?;
                    pipeline
                        .add_many([
                            &source,
                            &upload,
                            &gl_convert,
                            &download,
                            &convert,
                            appsink.upcast_ref(),
                        ])
                        .map_err(|err| CoreError::GStreamer(err.to_string()))?;

                    gst::Element::link_many([
                        &source,
                        &upload,
                        &gl_convert,
                        &download,
                        &convert,
                        appsink.upcast_ref(),
                    ])
                    .map_err(|err| CoreError::GStreamer(err.to_string()))?;
                }
            }

            Ok(AppsinkPipeline { pipeline, appsink })
        }
    }
}

fn rgb_caps(profile: CaptureProfile) -> gst::Caps {
    gst::Caps::builder("video/x-raw")
        .field("format", "RGB")
        .field("width", profile.width)
        .field("height", profile.height)
        .field("framerate", gst::Fraction::new(profile.fps, 1))
        .build()
}

fn make_element(factory: &'static str) -> Result<gst::Element> {
    gst::ElementFactory::make(factory)
        .build()
        .map_err(|_| CoreError::MissingElement(factory))
}

fn average_sample_rgb(sample: &gst::Sample) -> Result<RgbAverage> {
    let buffer = sample
        .buffer()
        .ok_or_else(|| CoreError::GStreamer("sample has no buffer".into()))?;
    let (width, height, layout) = sample_pixel_layout(sample)?;
    let map = buffer
        .map_readable()
        .map_err(|err| CoreError::GStreamer(err.to_string()))?;
    average_rgb_region(map.as_slice(), width, height, 0, width, layout)
}

fn average_sample_horizontal_bands(
    sample: &gst::Sample,
    bands: usize,
    profile: CaptureProfile,
) -> Result<Vec<RgbAverage>> {
    let buffer = sample
        .buffer()
        .ok_or_else(|| CoreError::GStreamer("sample has no buffer".into()))?;
    let (width, height, layout) = sample_pixel_layout(sample)?;
    let map = buffer
        .map_readable()
        .map_err(|err| CoreError::GStreamer(err.to_string()))?;
    average_rgb_horizontal_bands_sampled(
        map.as_slice(),
        width,
        height,
        bands,
        layout,
        profile.width as usize,
        profile.height as usize,
    )
}

fn average_sample_points(
    sample: &gst::Sample,
    points: &[SamplePoint],
    profile: CaptureProfile,
) -> Result<Vec<RgbAverage>> {
    let buffer = sample
        .buffer()
        .ok_or_else(|| CoreError::GStreamer("sample has no buffer".into()))?;
    let (width, height, layout) = sample_pixel_layout(sample)?;
    let map = buffer
        .map_readable()
        .map_err(|err| CoreError::GStreamer(err.to_string()))?;
    average_rgb_sample_points(
        map.as_slice(),
        width,
        height,
        layout,
        profile.width as usize,
        profile.height as usize,
        points,
    )
}

fn average_sample_regions(
    sample: &gst::Sample,
    regions: &[SampleRegion],
    profile: CaptureProfile,
) -> Result<Vec<RgbAverage>> {
    let buffer = sample
        .buffer()
        .ok_or_else(|| CoreError::GStreamer("sample has no buffer".into()))?;
    let (width, height, layout) = sample_pixel_layout(sample)?;
    let map = buffer
        .map_readable()
        .map_err(|_| CoreError::GStreamer("failed to map sample buffer".into()))?;
    average_rgb_sample_regions(
        map.as_slice(),
        width,
        height,
        layout,
        profile.width as usize,
        profile.height as usize,
        regions,
    )
}

fn detect_sample_black_bars(
    sample: &gst::Sample,
    threshold: u8,
    profile: CaptureProfile,
) -> Result<DetectedSampleCrop> {
    let buffer = sample
        .buffer()
        .ok_or_else(|| CoreError::GStreamer("sample has no buffer".into()))?;
    let (width, height, layout) = sample_pixel_layout(sample)?;
    let map = buffer
        .map_readable()
        .map_err(|err| CoreError::GStreamer(err.to_string()))?;
    detect_black_bars_sampled(
        map.as_slice(),
        width,
        height,
        layout,
        profile.width as usize,
        profile.height as usize,
        threshold,
    )
}

fn sample_pixel_layout(sample: &gst::Sample) -> Result<(usize, usize, PixelLayout)> {
    let caps = sample
        .caps()
        .ok_or_else(|| CoreError::GStreamer("sample has no caps".into()))?;
    let structure = caps
        .structure(0)
        .ok_or_else(|| CoreError::GStreamer("sample caps have no structure".into()))?;
    let format = structure
        .get::<String>("format")
        .map_err(|err| CoreError::GStreamer(format!("sample caps missing format: {err}")))?;
    let layout = PixelLayout::from_format(&format)?;

    let width = structure
        .get::<i32>("width")
        .map_err(|err| CoreError::GStreamer(format!("sample caps missing width: {err}")))?;
    let height = structure
        .get::<i32>("height")
        .map_err(|err| CoreError::GStreamer(format!("sample caps missing height: {err}")))?;
    if width <= 0 || height <= 0 {
        return Err(CoreError::GStreamer(format!(
            "invalid sample dimensions: {width}x{height}"
        )));
    }

    Ok((width as usize, height as usize, layout))
}

#[derive(Debug, Clone, Copy)]
struct PixelLayout {
    bytes_per_pixel: usize,
    red_offset: usize,
    green_offset: usize,
    blue_offset: usize,
}

impl PixelLayout {
    const RGB: Self = Self {
        bytes_per_pixel: 3,
        red_offset: 0,
        green_offset: 1,
        blue_offset: 2,
    };

    fn from_format(format: &str) -> Result<Self> {
        match format {
            "RGB" => Ok(Self::RGB),
            "BGR" => Ok(Self {
                bytes_per_pixel: 3,
                red_offset: 2,
                green_offset: 1,
                blue_offset: 0,
            }),
            "RGBx" | "RGBA" => Ok(Self {
                bytes_per_pixel: 4,
                red_offset: 0,
                green_offset: 1,
                blue_offset: 2,
            }),
            "BGRx" | "BGRA" => Ok(Self {
                bytes_per_pixel: 4,
                red_offset: 2,
                green_offset: 1,
                blue_offset: 0,
            }),
            "xRGB" | "ARGB" => Ok(Self {
                bytes_per_pixel: 4,
                red_offset: 1,
                green_offset: 2,
                blue_offset: 3,
            }),
            "xBGR" | "ABGR" => Ok(Self {
                bytes_per_pixel: 4,
                red_offset: 3,
                green_offset: 2,
                blue_offset: 1,
            }),
            _ => Err(CoreError::GStreamer(format!(
                "unsupported sample format: {format}"
            ))),
        }
    }
}

#[cfg(test)]
fn average_rgb_bytes(bytes: &[u8]) -> Result<RgbAverage> {
    let chunks = bytes.chunks_exact(3);
    let pixels = chunks.len();
    if pixels == 0 {
        return Err(CoreError::GStreamer("empty RGB buffer".into()));
    }

    let mut red = 0u64;
    let mut green = 0u64;
    let mut blue = 0u64;

    for pixel in chunks {
        red += u64::from(pixel[0]);
        green += u64::from(pixel[1]);
        blue += u64::from(pixel[2]);
    }

    Ok(RgbAverage {
        red: (red / pixels as u64) as u8,
        green: (green / pixels as u64) as u8,
        blue: (blue / pixels as u64) as u8,
    })
}

#[cfg(test)]
fn average_rgb_horizontal_bands(
    bytes: &[u8],
    width: usize,
    height: usize,
    bands: usize,
    layout: PixelLayout,
) -> Result<Vec<RgbAverage>> {
    if width == 0 || height == 0 || bands == 0 {
        return Err(CoreError::GStreamer(format!(
            "invalid RGB bands request: {width}x{height}, bands={bands}"
        )));
    }

    let expected_len = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(layout.bytes_per_pixel))
        .ok_or_else(|| CoreError::GStreamer("RGB frame dimensions overflowed".into()))?;
    if bytes.len() < expected_len {
        return Err(CoreError::GStreamer(format!(
            "RGB frame too small: got {} bytes, expected at least {expected_len}",
            bytes.len()
        )));
    }

    let mut averages = Vec::with_capacity(bands);
    for band in 0..bands {
        let start_x = band * width / bands;
        let end_x = (band + 1) * width / bands;
        averages.push(average_rgb_region(
            bytes, width, height, start_x, end_x, layout,
        )?);
    }

    Ok(averages)
}

fn average_rgb_horizontal_bands_sampled(
    bytes: &[u8],
    width: usize,
    height: usize,
    bands: usize,
    layout: PixelLayout,
    sample_width: usize,
    sample_height: usize,
) -> Result<Vec<RgbAverage>> {
    if width == 0 || height == 0 || bands == 0 || sample_width == 0 || sample_height == 0 {
        return Err(CoreError::GStreamer(format!(
            "invalid sampled RGB bands request: {width}x{height}, bands={bands}, sample={sample_width}x{sample_height}"
        )));
    }

    validate_frame_len(bytes, width, height, layout)?;

    let sample_width = sample_width.min(width).max(bands);
    let sample_height = sample_height.min(height);
    let mut averages = Vec::with_capacity(bands);

    for band in 0..bands {
        let start_sx = band * sample_width / bands;
        let end_sx = (band + 1) * sample_width / bands;
        averages.push(average_rgb_sampled_band(
            bytes,
            width,
            height,
            start_sx,
            end_sx,
            sample_width,
            sample_height,
            layout,
        )?);
    }

    Ok(averages)
}

fn average_rgb_sample_points(
    bytes: &[u8],
    width: usize,
    height: usize,
    layout: PixelLayout,
    sample_width: usize,
    sample_height: usize,
    points: &[SamplePoint],
) -> Result<Vec<RgbAverage>> {
    if width == 0 || height == 0 || sample_width == 0 || sample_height == 0 {
        return Err(CoreError::GStreamer(format!(
            "invalid sampled RGB points request: {width}x{height}, sample={sample_width}x{sample_height}"
        )));
    }

    validate_frame_len(bytes, width, height, layout)?;

    let sample_width = sample_width.min(width);
    let sample_height = sample_height.min(height);
    let (radius_x, radius_y) = point_sample_radius(sample_width, sample_height);

    points
        .iter()
        .map(|point| {
            average_rgb_sample_point(
                bytes,
                width,
                height,
                layout,
                sample_width,
                sample_height,
                *point,
                radius_x,
                radius_y,
            )
        })
        .collect()
}

fn average_rgb_sample_regions(
    bytes: &[u8],
    width: usize,
    height: usize,
    layout: PixelLayout,
    sample_width: usize,
    sample_height: usize,
    regions: &[SampleRegion],
) -> Result<Vec<RgbAverage>> {
    if width == 0 || height == 0 || sample_width == 0 || sample_height == 0 {
        return Err(CoreError::GStreamer(format!(
            "invalid sampled RGB regions request: {width}x{height}, sample={sample_width}x{sample_height}"
        )));
    }

    validate_frame_len(bytes, width, height, layout)?;

    let sample_width = sample_width.min(width);
    let sample_height = sample_height.min(height);

    regions
        .iter()
        .map(|region| {
            average_rgb_sample_region(
                bytes,
                width,
                height,
                layout,
                sample_width,
                sample_height,
                *region,
            )
        })
        .collect()
}

fn point_sample_radius(sample_width: usize, sample_height: usize) -> (usize, usize) {
    ((sample_width / 24).max(1), (sample_height / 14).max(1))
}

fn detect_black_bars_sampled(
    bytes: &[u8],
    width: usize,
    height: usize,
    layout: PixelLayout,
    sample_width: usize,
    sample_height: usize,
    threshold: u8,
) -> Result<DetectedSampleCrop> {
    if width == 0 || height == 0 || sample_width == 0 || sample_height == 0 {
        return Err(CoreError::GStreamer(format!(
            "invalid black bar detection request: {width}x{height}, sample={sample_width}x{sample_height}"
        )));
    }

    validate_frame_len(bytes, width, height, layout)?;

    let sample_width = sample_width.min(width);
    let sample_height = sample_height.min(height);
    let top = count_dark_rows_from_top(
        bytes,
        width,
        height,
        layout,
        sample_width,
        sample_height,
        threshold,
    );
    let bottom = count_dark_rows_from_bottom(
        bytes,
        width,
        height,
        layout,
        sample_width,
        sample_height,
        threshold,
    );
    let left = count_dark_cols_from_left(
        bytes,
        width,
        height,
        layout,
        sample_width,
        sample_height,
        threshold,
    );
    let right = count_dark_cols_from_right(
        bytes,
        width,
        height,
        layout,
        sample_width,
        sample_height,
        threshold,
    );

    if top + bottom >= sample_height || left + right >= sample_width {
        return Ok(DetectedSampleCrop::NONE);
    }

    Ok(DetectedSampleCrop {
        left: left as f64 / sample_width as f64,
        right: right as f64 / sample_width as f64,
        top: top as f64 / sample_height as f64,
        bottom: bottom as f64 / sample_height as f64,
    })
}

#[allow(clippy::too_many_arguments)]
fn count_dark_rows_from_top(
    bytes: &[u8],
    width: usize,
    height: usize,
    layout: PixelLayout,
    sample_width: usize,
    sample_height: usize,
    threshold: u8,
) -> usize {
    (0..sample_height)
        .take_while(|&sy| {
            sampled_row_luma(
                bytes,
                width,
                height,
                layout,
                sample_width,
                sample_height,
                sy,
            ) <= threshold
        })
        .count()
}

#[allow(clippy::too_many_arguments)]
fn count_dark_rows_from_bottom(
    bytes: &[u8],
    width: usize,
    height: usize,
    layout: PixelLayout,
    sample_width: usize,
    sample_height: usize,
    threshold: u8,
) -> usize {
    (0..sample_height)
        .rev()
        .take_while(|&sy| {
            sampled_row_luma(
                bytes,
                width,
                height,
                layout,
                sample_width,
                sample_height,
                sy,
            ) <= threshold
        })
        .count()
}

#[allow(clippy::too_many_arguments)]
fn count_dark_cols_from_left(
    bytes: &[u8],
    width: usize,
    height: usize,
    layout: PixelLayout,
    sample_width: usize,
    sample_height: usize,
    threshold: u8,
) -> usize {
    (0..sample_width)
        .take_while(|&sx| {
            sampled_col_luma(
                bytes,
                width,
                height,
                layout,
                sample_width,
                sample_height,
                sx,
            ) <= threshold
        })
        .count()
}

#[allow(clippy::too_many_arguments)]
fn count_dark_cols_from_right(
    bytes: &[u8],
    width: usize,
    height: usize,
    layout: PixelLayout,
    sample_width: usize,
    sample_height: usize,
    threshold: u8,
) -> usize {
    (0..sample_width)
        .rev()
        .take_while(|&sx| {
            sampled_col_luma(
                bytes,
                width,
                height,
                layout,
                sample_width,
                sample_height,
                sx,
            ) <= threshold
        })
        .count()
}

fn sampled_row_luma(
    bytes: &[u8],
    width: usize,
    height: usize,
    layout: PixelLayout,
    sample_width: usize,
    sample_height: usize,
    sy: usize,
) -> u8 {
    let y = sy * height / sample_height;
    let total = (0..sample_width)
        .map(|sx| {
            let x = sx * width / sample_width;
            pixel_luma(bytes, width, layout, x, y)
        })
        .sum::<u64>();

    (total / sample_width as u64) as u8
}

fn sampled_col_luma(
    bytes: &[u8],
    width: usize,
    height: usize,
    layout: PixelLayout,
    sample_width: usize,
    sample_height: usize,
    sx: usize,
) -> u8 {
    let x = sx * width / sample_width;
    let total = (0..sample_height)
        .map(|sy| {
            let y = sy * height / sample_height;
            pixel_luma(bytes, width, layout, x, y)
        })
        .sum::<u64>();

    (total / sample_height as u64) as u8
}

fn pixel_luma(bytes: &[u8], width: usize, layout: PixelLayout, x: usize, y: usize) -> u64 {
    let offset = (y * width + x) * layout.bytes_per_pixel;
    let red = u64::from(bytes[offset + layout.red_offset]);
    let green = u64::from(bytes[offset + layout.green_offset]);
    let blue = u64::from(bytes[offset + layout.blue_offset]);
    (red * 54 + green * 183 + blue * 19) / 256
}

#[allow(clippy::too_many_arguments)]
fn average_rgb_sample_point(
    bytes: &[u8],
    width: usize,
    height: usize,
    layout: PixelLayout,
    sample_width: usize,
    sample_height: usize,
    point: SamplePoint,
    radius_x: usize,
    radius_y: usize,
) -> Result<RgbAverage> {
    let center_sx = normalized_to_sample_index(point.x, sample_width);
    let center_sy = normalized_to_sample_index(point.y, sample_height);
    let start_sx = center_sx.saturating_sub(radius_x);
    let end_sx = center_sx.saturating_add(radius_x).min(sample_width - 1);
    let start_sy = center_sy.saturating_sub(radius_y);
    let end_sy = center_sy.saturating_add(radius_y).min(sample_height - 1);

    let mut red = 0u64;
    let mut green = 0u64;
    let mut blue = 0u64;
    let mut pixels = 0u64;

    for sy in start_sy..=end_sy {
        let y = sy * height / sample_height;
        for sx in start_sx..=end_sx {
            let x = sx * width / sample_width;
            let offset = (y * width + x) * layout.bytes_per_pixel;
            red += u64::from(bytes[offset + layout.red_offset]);
            green += u64::from(bytes[offset + layout.green_offset]);
            blue += u64::from(bytes[offset + layout.blue_offset]);
            pixels += 1;
        }
    }

    Ok(RgbAverage {
        red: (red / pixels) as u8,
        green: (green / pixels) as u8,
        blue: (blue / pixels) as u8,
    })
}

#[allow(clippy::too_many_arguments)]
fn average_rgb_sample_region(
    bytes: &[u8],
    width: usize,
    height: usize,
    layout: PixelLayout,
    sample_width: usize,
    sample_height: usize,
    region: SampleRegion,
) -> Result<RgbAverage> {
    let center_sx = normalized_to_sample_index(region.center.x, sample_width);
    let center_sy = normalized_to_sample_index(region.center.y, sample_height);
    let radius_x = ((region.width * sample_width as f64) / 2.0).round() as usize;
    let radius_y = ((region.height * sample_height as f64) / 2.0).round() as usize;
    let start_sx = center_sx.saturating_sub(radius_x).min(sample_width - 1);
    let end_sx = center_sx.saturating_add(radius_x).min(sample_width - 1);
    let start_sy = center_sy.saturating_sub(radius_y).min(sample_height - 1);
    let end_sy = center_sy.saturating_add(radius_y).min(sample_height - 1);

    let mut red = 0u64;
    let mut green = 0u64;
    let mut blue = 0u64;
    let mut weight_sum = 0u64;

    for sy in start_sy..=end_sy {
        let y = sy * height / sample_height;
        for sx in start_sx..=end_sx {
            let x = sx * width / sample_width;
            let distance_x = sx.abs_diff(center_sx);
            let distance_y = sy.abs_diff(center_sy);
            let weight_x = radius_x.saturating_add(1).saturating_sub(distance_x).max(1);
            let weight_y = radius_y.saturating_add(1).saturating_sub(distance_y).max(1);
            let weight = (weight_x * weight_y) as u64;
            let offset = (y * width + x) * layout.bytes_per_pixel;
            red += u64::from(bytes[offset + layout.red_offset]) * weight;
            green += u64::from(bytes[offset + layout.green_offset]) * weight;
            blue += u64::from(bytes[offset + layout.blue_offset]) * weight;
            weight_sum += weight;
        }
    }

    Ok(RgbAverage {
        red: (red / weight_sum) as u8,
        green: (green / weight_sum) as u8,
        blue: (blue / weight_sum) as u8,
    })
}

fn normalized_to_sample_index(value: f64, sample_size: usize) -> usize {
    if sample_size <= 1 {
        return 0;
    }

    (value.clamp(0.0, 1.0) * (sample_size - 1) as f64).round() as usize
}

fn validate_frame_len(
    bytes: &[u8],
    width: usize,
    height: usize,
    layout: PixelLayout,
) -> Result<()> {
    let expected_len = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(layout.bytes_per_pixel))
        .ok_or_else(|| CoreError::GStreamer("RGB frame dimensions overflowed".into()))?;
    if bytes.len() < expected_len {
        return Err(CoreError::GStreamer(format!(
            "RGB frame too small: got {} bytes, expected at least {expected_len}",
            bytes.len()
        )));
    }

    Ok(())
}

fn average_rgb_sampled_band(
    bytes: &[u8],
    width: usize,
    height: usize,
    start_sx: usize,
    end_sx: usize,
    sample_width: usize,
    sample_height: usize,
    layout: PixelLayout,
) -> Result<RgbAverage> {
    if start_sx >= end_sx || end_sx > sample_width {
        return Err(CoreError::GStreamer(format!(
            "invalid sampled RGB band: start_sx={start_sx}, end_sx={end_sx}, sample_width={sample_width}"
        )));
    }

    let mut red = 0u64;
    let mut green = 0u64;
    let mut blue = 0u64;
    let mut pixels = 0u64;

    for sy in 0..sample_height {
        let y = sy * height / sample_height;
        for sx in start_sx..end_sx {
            let x = sx * width / sample_width;
            let offset = (y * width + x) * layout.bytes_per_pixel;
            red += u64::from(bytes[offset + layout.red_offset]);
            green += u64::from(bytes[offset + layout.green_offset]);
            blue += u64::from(bytes[offset + layout.blue_offset]);
            pixels += 1;
        }
    }

    Ok(RgbAverage {
        red: (red / pixels) as u8,
        green: (green / pixels) as u8,
        blue: (blue / pixels) as u8,
    })
}

fn average_rgb_region(
    bytes: &[u8],
    width: usize,
    height: usize,
    start_x: usize,
    end_x: usize,
    layout: PixelLayout,
) -> Result<RgbAverage> {
    if start_x >= end_x || end_x > width {
        return Err(CoreError::GStreamer(format!(
            "invalid RGB region: start_x={start_x}, end_x={end_x}, width={width}"
        )));
    }

    let mut red = 0u64;
    let mut green = 0u64;
    let mut blue = 0u64;
    let mut pixels = 0u64;

    for y in 0..height {
        for x in start_x..end_x {
            let offset = (y * width + x) * layout.bytes_per_pixel;
            red += u64::from(bytes[offset + layout.red_offset]);
            green += u64::from(bytes[offset + layout.green_offset]);
            blue += u64::from(bytes[offset + layout.blue_offset]);
            pixels += 1;
        }
    }

    Ok(RgbAverage {
        red: (red / pixels) as u8,
        green: (green / pixels) as u8,
        blue: (blue / pixels) as u8,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        average_rgb_bytes, average_rgb_horizontal_bands, average_rgb_horizontal_bands_sampled,
        average_rgb_sample_points, average_rgb_sample_regions, detect_black_bars_sampled,
        DetectedSampleCrop, PixelLayout, RgbAverage, SamplePoint, SampleRegion,
    };

    #[test]
    fn averages_rgb_bytes() {
        let average = average_rgb_bytes(&[255, 0, 0, 0, 255, 0, 0, 0, 255]).unwrap();
        assert_eq!(
            average,
            RgbAverage {
                red: 85,
                green: 85,
                blue: 85,
            }
        );
    }

    #[test]
    fn rejects_empty_rgb_buffer() {
        assert!(average_rgb_bytes(&[]).is_err());
    }

    #[test]
    fn averages_horizontal_rgb_bands() {
        let bytes = [
            255, 0, 0, 255, 0, 0, 0, 0, 255, 0, 0, 255, // row 1
            255, 0, 0, 255, 0, 0, 0, 0, 255, 0, 0, 255, // row 2
        ];

        let averages = average_rgb_horizontal_bands(&bytes, 4, 2, 2, PixelLayout::RGB).unwrap();

        assert_eq!(
            averages,
            vec![
                RgbAverage {
                    red: 255,
                    green: 0,
                    blue: 0
                },
                RgbAverage {
                    red: 0,
                    green: 0,
                    blue: 255
                }
            ]
        );
    }

    #[test]
    fn rejects_too_small_rgb_frame_for_bands() {
        assert!(average_rgb_horizontal_bands(&[255, 0, 0], 2, 1, 1, PixelLayout::RGB).is_err());
    }

    #[test]
    fn averages_bgrx_horizontal_bands() {
        let layout = PixelLayout::from_format("BGRx").unwrap();
        let bytes = [0, 0, 255, 0, 0, 0, 255, 0, 255, 0, 0, 0, 255, 0, 0, 0];

        let averages = average_rgb_horizontal_bands(&bytes, 4, 1, 2, layout).unwrap();

        assert_eq!(
            averages,
            vec![
                RgbAverage {
                    red: 255,
                    green: 0,
                    blue: 0
                },
                RgbAverage {
                    red: 0,
                    green: 0,
                    blue: 255
                }
            ]
        );
    }

    #[test]
    fn sampled_horizontal_bands_use_requested_grid() {
        let bytes = [
            255, 0, 0, 10, 0, 0, 0, 0, 255, 0, 0, 10, // row 1
            255, 0, 0, 10, 0, 0, 0, 0, 255, 0, 0, 10, // row 2
            255, 0, 0, 10, 0, 0, 0, 0, 255, 0, 0, 10, // row 3
            255, 0, 0, 10, 0, 0, 0, 0, 255, 0, 0, 10, // row 4
        ];

        let averages =
            average_rgb_horizontal_bands_sampled(&bytes, 4, 4, 2, PixelLayout::RGB, 2, 2).unwrap();

        assert_eq!(
            averages,
            vec![
                RgbAverage {
                    red: 255,
                    green: 0,
                    blue: 0
                },
                RgbAverage {
                    red: 0,
                    green: 0,
                    blue: 255
                }
            ]
        );
    }

    #[test]
    fn sampled_horizontal_bands_reject_invalid_sample_grid() {
        assert!(average_rgb_horizontal_bands_sampled(
            &[255, 0, 0],
            1,
            1,
            1,
            PixelLayout::RGB,
            0,
            1
        )
        .is_err());
    }

    #[test]
    fn averages_sample_points_on_grid() {
        let bytes = [
            255, 0, 0, 255, 0, 0, 0, 255, 0, 0, 255, 0, // row 1
            255, 0, 0, 255, 0, 0, 0, 255, 0, 0, 255, 0, // row 2
            0, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255, 255, // row 3
            0, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255, 255, // row 4
        ];

        let averages = average_rgb_sample_points(
            &bytes,
            4,
            4,
            PixelLayout::RGB,
            4,
            4,
            &[SamplePoint::new(0.0, 0.0), SamplePoint::new(1.0, 1.0)],
        )
        .unwrap();

        assert_eq!(
            averages,
            vec![
                RgbAverage {
                    red: 255,
                    green: 0,
                    blue: 0
                },
                RgbAverage {
                    red: 255,
                    green: 255,
                    blue: 255
                }
            ]
        );
    }

    #[test]
    fn sample_point_clamps_to_unit_square() {
        assert_eq!(SamplePoint::new(-1.0, 2.0), SamplePoint { x: 0.0, y: 1.0 });
    }

    #[test]
    fn averages_weighted_sample_regions() {
        let bytes = [
            255, 0, 0, 255, 0, 0, 0, 0, 255, 0, 0, 255, // row 1
            255, 0, 0, 255, 0, 0, 0, 0, 255, 0, 0, 255, // row 2
            0, 255, 0, 0, 255, 0, 255, 255, 255, 255, 255, 255, // row 3
            0, 255, 0, 0, 255, 0, 255, 255, 255, 255, 255, 255, // row 4
        ];

        let averages = average_rgb_sample_regions(
            &bytes,
            4,
            4,
            PixelLayout::RGB,
            4,
            4,
            &[
                SampleRegion::new(SamplePoint::new(0.0, 0.0), 0.5, 0.5),
                SampleRegion::new(SamplePoint::new(1.0, 1.0), 0.5, 0.5),
            ],
        )
        .unwrap();

        assert_eq!(
            averages,
            vec![
                RgbAverage {
                    red: 255,
                    green: 0,
                    blue: 0,
                },
                RgbAverage {
                    red: 255,
                    green: 255,
                    blue: 255,
                }
            ]
        );
    }

    #[test]
    fn detects_black_bars_on_all_edges() {
        let mut bytes = vec![0u8; 6 * 6 * 3];
        for y in 1..5 {
            for x in 1..5 {
                let offset = (y * 6 + x) * 3;
                bytes[offset] = 255;
                bytes[offset + 1] = 255;
                bytes[offset + 2] = 255;
            }
        }

        let crop = detect_black_bars_sampled(&bytes, 6, 6, PixelLayout::RGB, 6, 6, 8).unwrap();
        assert_eq!(
            crop,
            DetectedSampleCrop {
                left: 1.0 / 6.0,
                right: 1.0 / 6.0,
                top: 1.0 / 6.0,
                bottom: 1.0 / 6.0,
            }
        );
    }

    #[test]
    fn black_bar_detection_ignores_dark_content_inside_bright_edges() {
        let mut bytes = vec![255u8; 5 * 5 * 3];
        let center = (2 * 5 + 2) * 3;
        bytes[center] = 0;
        bytes[center + 1] = 0;
        bytes[center + 2] = 0;

        let crop = detect_black_bars_sampled(&bytes, 5, 5, PixelLayout::RGB, 5, 5, 8).unwrap();
        assert_eq!(crop, DetectedSampleCrop::NONE);
    }

    #[test]
    fn black_bar_detection_ignores_fully_dark_frame() {
        let bytes = vec![0u8; 5 * 5 * 3];

        let crop = detect_black_bars_sampled(&bytes, 5, 5, PixelLayout::RGB, 5, 5, 8).unwrap();
        assert_eq!(crop, DetectedSampleCrop::NONE);
    }
}
