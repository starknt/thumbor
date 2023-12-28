use anyhow::{Ok, Result};
use base64::prelude::*;

mod abi;
pub use abi::*;
use photon_rs::transform::SamplingFilter;
use prost::Message;

impl ImageSpec {
    pub fn new(specs: Vec<Spec>) -> Self {
        Self { specs }
    }
}

impl From<&ImageSpec> for String {
    fn from(image_spec: &ImageSpec) -> Self {
        let data = image_spec.encode_to_vec();
        BASE64_STANDARD_NO_PAD.encode(data)
    }
}

impl TryFrom<&str> for ImageSpec {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let data = BASE64_STANDARD_NO_PAD.decode(s)?;
        Ok(ImageSpec::decode(&data[..])?)
    }
}

impl filter::Filter {
    pub fn to_str(self) -> Option<&'static str> {
        match self {
            filter::Filter::Unspecified => None,
            filter::Filter::Oceanic => Some("oceanic"),
            filter::Filter::Islands => Some("islands"),
            filter::Filter::Marine => Some("marine"),
        }
    }
}

impl From<resize::SampleFilter> for SamplingFilter {
    fn from(value: resize::SampleFilter) -> Self {
        match value {
            resize::SampleFilter::Undefined => Self::Nearest,
            resize::SampleFilter::Nearest => Self::Nearest,
            resize::SampleFilter::Triangle => Self::Triangle,
            resize::SampleFilter::CatmullRom => Self::CatmullRom,
            resize::SampleFilter::Gaussian => Self::Gaussian,
            resize::SampleFilter::Lanczos3 => Self::Lanczos3,
        }
    }
}

impl Spec {
    pub fn new_resize_seam_carve(width: u32, height: u32) -> Self {
        Self {
            data: Some(spec::Data::Resize(Resize {
                width,
                height,
                rtype: resize::ResizeType::SeamCarve as i32,
                filter: resize::SampleFilter::Undefined as i32,
            })),
        }
    }

    pub fn new_resize(width: u32, height: u32, filter: resize::SampleFilter) -> Self {
        Self {
            data: Some(spec::Data::Resize(Resize {
                width,
                height,
                rtype: resize::ResizeType::Normal as i32,
                filter: filter as i32,
            })),
        }
    }

    pub fn new_filter(filter: filter::Filter) -> Self {
        Self {
            data: Some(spec::Data::Filter(Filter {
                filter: filter as i32,
            })),
        }
    }

    pub fn new_watermark(x: u32, y: u32, a: u32) -> Self {
        Self {
            data: Some(spec::Data::Watermark(Watermark { x, y, alpha: a })),
        }
    }

    pub fn new_draw_text(text: String, x: u32, y: u32) -> Self {
        Self {
            data: Some(spec::Data::Text(DrawText { text, x, y })),
        }
    }

    pub fn new_oil() -> Self {
        Self {
            data: Some(spec::Data::Oil(OilEffect {
                radius: 2,
                intensity: 50.0,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Borrow;

    use super::*;

    #[test]
    fn encode_spec_could_be_decoded() {
        let spec1 = Spec::new_resize(600, 600, resize::SampleFilter::CatmullRom);
        let spec2 = Spec::new_filter(filter::Filter::Marine);
        let image_spec = ImageSpec::new(vec![spec1, spec2]);
        let s: String = image_spec.borrow().into();
        println!("encoded: {}", s);
        assert_eq!(image_spec, s.as_str().try_into().unwrap());
    }
}
