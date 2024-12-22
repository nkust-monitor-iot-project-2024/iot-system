use std::sync::Arc;

use anyhow::Context as _;
use async_nats::Message;
use bytes::Bytes;
use image::ImageFormat;
use serde::Serialize;
use yolo_rs::{BoundingBox, image_to_yolo_input_tensor, inference, model::YoloModelSession};

#[derive(Debug, Clone)]
pub struct RecognitionPayload {
    pub frame_id: String,
    pub picture: Bytes,
    pub picture_type: ImageFormat,
}

impl TryFrom<Message> for RecognitionPayload {
    type Error = anyhow::Error;

    fn try_from(msg: Message) -> Result<Self, Self::Error> {
        let header_map = msg.headers.unwrap_or_default();
        let content_type = header_map.get("Content-Type").map(|ct| ct.to_string());

        if let Some(content_type) = content_type {
            if content_type != "image/png" {
                anyhow::bail!("unsupported content type: {content_type}");
            }
        } else {
            anyhow::bail!("missing content type in the message");
        }

        let frame_id = header_map
            .get("Frame-Id")
            .map(|frame_id| frame_id.to_string())
            .context("missing Frame-Id header")?;

        let picture = msg.payload;
        let picture_type = ImageFormat::Png;

        Ok(Self {
            frame_id,
            picture,
            picture_type,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecognitionResult {
    pub frame_id: String,
    pub label: String,
    pub confidence: f32,
    pub picture: Bytes,
}

#[derive(Clone)]
pub struct RecognitionWorker {
    yolo_model: Arc<YoloModelSession>,
}

impl RecognitionWorker {
    pub fn new(yolo_model: Arc<YoloModelSession>) -> Self {
        Self { yolo_model }
    }

    #[tracing::instrument(skip(self, picture))]
    pub fn recognize(
        &self,
        RecognitionPayload {
            frame_id,
            picture,
            picture_type,
        }: RecognitionPayload,
    ) -> anyhow::Result<Vec<RecognitionResult>> {
        tracing::info!("Recognizing frame {}…", frame_id);

        let image_reader = {
            let mut reader = image::ImageReader::new(std::io::Cursor::new(picture));
            reader.set_format(ImageFormat::Png);

            reader
        };

        let image = match image_reader.decode() {
            Ok(frame_data) => frame_data,
            Err(e) => {
                anyhow::bail!("Failed to decode image: {:?}", e);
            }
        };

        let yolo_input = image_to_yolo_input_tensor(&image);
        let yolo_output =
            inference(&self.yolo_model, yolo_input.view()).expect("failed to run inference");

        tracing::info!("Found {} entities", yolo_output.len());

        let results = yolo_output
            .into_iter()
            .map(|entity| {
                let BoundingBox { x1, x2, y1, y2 } = entity.bounding_box;
                let label = entity.label;
                let confidence = entity.confidence;

                let cropped_image =
                    image.crop_imm(x1 as _, y1 as _, (x2 - x1) as u32, (y2 - y1) as u32);

                // encode the cropped image to PNG
                let mut buf = Vec::new();
                let mut cursor = std::io::Cursor::new(&mut buf);
                cropped_image
                    .write_to(&mut cursor, ImageFormat::Png)
                    .context("Failed to write cropped image to PNG")?;

                Ok(RecognitionResult {
                    frame_id: frame_id.clone(),
                    label: label.to_string(), // fixme: leverage ArcStr
                    confidence,
                    picture: Bytes::from(buf),
                })
            })
            .collect::<anyhow::Result<Vec<RecognitionResult>>>()?;

        tracing::info!("Recognized! Found {} entities.", results.len());
        Ok(results)
    }
}