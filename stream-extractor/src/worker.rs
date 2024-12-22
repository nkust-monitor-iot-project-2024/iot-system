use anyhow::Context;
use glib::object::Cast;
use gst::prelude::*;
use gstreamer::prelude::ElementExt;
use gstreamer::{self as gst, Pipeline};
use gstreamer_app::AppSinkCallbacks;
use gstreamer_video as gst_video;
use image::{DynamicImage, RgbImage};
use std::sync::atomic::{AtomicUsize, Ordering};

/// The builder of the extractor worker pipeline.
pub struct ExtractorWorkerBuilder {
    pub rtsp_url: String,

    /// The sender to the queue to NATS.
    ///
    /// The first element of the tuple is the frame counter, and the second element is the frame.
    pub sender: crossbeam::channel::Sender<(usize, DynamicImage)>,

    /// Specifies the interval at which frames are dispatched to the inference queue.
    ///
    /// This parameter helps manage the load on the inference queue by sending
    /// frames only after a specified number of frames have been processed.
    /// The default is set to dispatch a frame every 300 frames (10s at 30fps, 5s at 60fps).
    pub frame_interval: Option<usize>,
}

impl ExtractorWorkerBuilder {
    pub fn build(self) -> anyhow::Result<Pipeline> {
        let pipeline = gstreamer::Pipeline::new();
        let frame_interval = self.frame_interval.unwrap_or(300);

        let rtspsrc_element = gst::ElementFactory::make("rtspsrc")
            .property("location", self.rtsp_url)
            .build()
            .context("failed to create rtspsrc element")?;

        let rtpjitterbuffer_element = gst::ElementFactory::make("rtpjitterbuffer")
            .build()
            .context("failed to create rtpjitterbuffer element")?;

        let rtph264depay_element = gst::ElementFactory::make("rtph264depay")
            .property("wait-for-keyframe", true)
            .property("request-keyframe", true)
            .build()
            .context("failed to create rtph264depay element")?;

        let avdec_h264_element = gst::ElementFactory::make("avdec_h264")
            .build()
            .context("failed to create avdec_h264 element")?;

        let videoconvert_element = gst::ElementFactory::make("videoconvert")
            .build()
            .context("failed to create videoconvert element")?;

        let identity_element = gst::ElementFactory::make("identity")
            .property("check-imperfect-offset", true)
            .property("check-imperfect-timestamp", true)
            .build()
            .context("failed to create identity element")?;

        let frame_counter = AtomicUsize::new(0);

        let appsink_callback = AppSinkCallbacks::builder()
            .new_sample(move |sink| {
                let sample = match sink.pull_sample() {
                    Ok(sample) => sample,
                    Err(_) => return Err(gst::FlowError::Error),
                };

                // Extract the buffer and caps (metadata)
                let buffer = sample.buffer().unwrap();
                let caps = sample.caps().unwrap();
                let video_info = gst_video::VideoInfo::from_caps(caps).unwrap();

                // Convert the buffer to a readable format
                let map = buffer.map_readable().unwrap();

                // Increment the frame counter
                let counter = frame_counter.fetch_add(1, Ordering::Relaxed);

                // Save frame as WebP every second (assuming 1 frame per second)
                if counter % frame_interval == 0 {
                    let width = video_info.width() as usize;
                    let height = video_info.height() as usize;

                    // Extract the frame data
                    let frame_data = map.as_slice();

                    let frame =
                        RgbImage::from_raw(width as u32, height as u32, frame_data.to_vec())
                            .expect("expect a valid image");
                    let dynamic_image = DynamicImage::ImageRgb8(frame);

                    self.sender
                        .send((counter, dynamic_image))
                        .expect("failed to send frame to inferrence queue");
                }

                Ok(gst::FlowSuccess::Ok)
            })
            .build();

        let appsink_element = gstreamer_app::AppSink::builder()
            .name("appsink")
            .sync(true)
            .callbacks(appsink_callback)
            .caps(
                &gst::Caps::builder("video/x-raw")
                    .field("format", "RGB")
                    .build(),
            )
            .build()
            .upcast();

        pipeline.add_many([
            &rtspsrc_element,
            &rtpjitterbuffer_element,
            &rtph264depay_element,
            &avdec_h264_element,
            &videoconvert_element,
            &identity_element,
            &appsink_element,
        ])?;

        let rtpjitterbuffer_element_clone = rtpjitterbuffer_element.clone();
        rtspsrc_element.connect_pad_added(move |_, src_pad| {
            let sink_pad = rtpjitterbuffer_element_clone.static_pad("sink").unwrap();
            if !sink_pad.is_linked() {
                match src_pad.link(&sink_pad) {
                    Ok(_) => tracing::info!("Successfully linked pads"),
                    Err(err) => tracing::warn!("Failed to link pads: {:?}", err),
                }
            }
        });

        // link elements
        gst::Element::link_many([
            &rtpjitterbuffer_element,
            &rtph264depay_element,
            &avdec_h264_element,
            &videoconvert_element,
            &identity_element,
            &appsink_element,
        ])?;

        Ok(pipeline)
    }
}
