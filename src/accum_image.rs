use std::ops::{Deref, DerefMut};

use crate::image::Image;

/// Specialized image type where each image pixel represents an average of all
/// accumulated luminance values. The amount of times sampled is stored to
/// allow recalculating this average once we have a new sample.
pub struct AccumulatedImage {
    pub times_sampled: usize,
    pub image: Image,
}

impl AccumulatedImage {
    pub fn new(extent: (usize, usize)) -> Self {
        Self {
            times_sampled: 0,
            image: Image::new(extent),
        }
    }
}

impl Deref for AccumulatedImage {
    type Target = Image;

    fn deref(&self) -> &Self::Target {
        &self.image
    }
}

impl DerefMut for AccumulatedImage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.image
    }
}
