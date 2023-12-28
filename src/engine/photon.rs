use std::io::Cursor;

use crate::pb::{
    filter, resize, spec, Contrast, Crop, DrawText, Filter, Fliph, Flipv, OilEffect, Resize,
    Watermark,
};
use anyhow::Result;
use bytes::Bytes;
use image::{DynamicImage, ImageBuffer, ImageOutputFormat};
use lazy_static::lazy_static;
use photon_rs::{
    effects, filters, multiple,
    native::open_image_from_bytes,
    text,
    transform::{self},
    PhotonImage,
};

use super::{Engine, SpecTransformer};

lazy_static! {
    static ref WATERMARK: PhotonImage = {
        let data = include_bytes!("../../rust-logo.png");
        let watermark = open_image_from_bytes(data).unwrap();
        transform::resize(&watermark, 64, 64, transform::SamplingFilter::Nearest)
    };
}

pub struct Photon(PhotonImage);

impl TryFrom<Bytes> for Photon {
    type Error = anyhow::Error;

    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        Ok(Self(open_image_from_bytes(&value)?))
    }
}

impl Engine for Photon {
    fn apply(&mut self, specs: &[crate::pb::Spec]) {
        for spec in specs.iter() {
            match spec.data {
                Some(spec::Data::Crop(ref v)) => self.transform(v),
                Some(spec::Data::Resize(ref v)) => self.transform(v),
                Some(spec::Data::Contrast(ref v)) => self.transform(v),
                Some(spec::Data::Filter(ref v)) => self.transform(v),
                Some(spec::Data::Fliph(ref v)) => self.transform(v),
                Some(spec::Data::Flipv(ref v)) => self.transform(v),
                Some(spec::Data::Watermark(ref v)) => self.transform(v),
                Some(spec::Data::Text(ref v)) => self.transform(v),
                Some(spec::Data::Oil(ref v)) => self.transform(v),
                _ => {}
            }
        }
    }

    fn generate(self, format: image::ImageOutputFormat) -> Vec<u8> {
        image_to_bytes(self.0, format)
    }
}

impl SpecTransformer<&Fliph> for Photon {
    fn transform(&mut self, _op: &Fliph) {
        transform::fliph(&mut self.0);
    }
}

impl SpecTransformer<&Flipv> for Photon {
    fn transform(&mut self, _op: &Flipv) {
        transform::flipv(&mut self.0);
    }
}

impl SpecTransformer<&Watermark> for Photon {
    fn transform(&mut self, op: &Watermark) {
        multiple::watermark(&mut self.0, &WATERMARK, op.x, op.y);
    }
}

impl SpecTransformer<&Crop> for Photon {
    fn transform(&mut self, op: &Crop) {
        let img = transform::crop(&mut self.0, op.x1, op.y1, op.x2, op.y2);
        self.0 = img;
    }
}

impl SpecTransformer<&Resize> for Photon {
    fn transform(&mut self, op: &Resize) {
        let img = match resize::ResizeType::from_i32(op.rtype).unwrap() {
            resize::ResizeType::Normal => transform::resize(
                &self.0,
                op.width,
                op.height,
                resize::SampleFilter::from_i32(op.filter).unwrap().into(),
            ),
            resize::ResizeType::SeamCarve => transform::seam_carve(&self.0, op.width, op.height),
        };

        self.0 = img;
    }
}

impl SpecTransformer<&Contrast> for Photon {
    fn transform(&mut self, op: &Contrast) {
        effects::adjust_contrast(&mut self.0, op.contrast);
    }
}

impl SpecTransformer<&Filter> for Photon {
    fn transform(&mut self, op: &Filter) {
        match filter::Filter::from_i32(op.filter) {
            Some(filter::Filter::Unspecified) => {}
            Some(f) => filters::filter(&mut self.0, f.to_str().unwrap()),
            _ => {}
        }
    }
}

impl SpecTransformer<&DrawText> for Photon {
    fn transform(&mut self, op: &DrawText) {
        text::draw_text(&mut self.0, op.text.as_str(), op.x, op.y)
    }
}

impl SpecTransformer<&OilEffect> for Photon {
    fn transform(&mut self, op: &OilEffect) {
        effects::oil(&mut self.0, op.radius, op.intensity as f64)
    }
}

fn image_to_bytes(img: PhotonImage, format: ImageOutputFormat) -> Vec<u8> {
    let raw_pixels = img.get_raw_pixels();
    let width = img.get_width();
    let height = img.get_height();

    let img_buf = ImageBuffer::from_vec(width, height, raw_pixels).unwrap();
    let dynimage = DynamicImage::ImageRgba8(img_buf);

    let mut buffer = Cursor::new(Vec::with_capacity(32768));
    dynimage.write_to(&mut buffer, format).unwrap();
    buffer.into_inner()
}
