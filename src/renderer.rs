use std::sync::Arc;

use rand::Rng;
use tracing::{debug, trace};

use crate::{peer::PeerTable, render_image::RenderImage};

pub struct Renderer {
    pub peer_table: PeerTable,
    pub render_image: RenderImage,
}

impl Renderer {
    pub fn new(pt: PeerTable) -> Self {
        Self {
            peer_table: pt,
            render_image: RenderImage::new((400, 300)),
        }
    }

    pub fn render_tile(&self, tile_size: (usize, usize)) -> Vec<u8> {
        let mut res = Vec::with_capacity(tile_size.0 * tile_size.1 * 3);
        let mut rng = rand::rng();
        let random_rbg: [u8; _] = [rng.random(), rng.random(), rng.random()];

        for _ in 0..tile_size.1 {
            for _ in 0..tile_size.0 {
                res.push(random_rbg[0]);
                res.push(random_rbg[1]);
                res.push(random_rbg[2]);
            }
        }

        res
    }
}

fn image_insert_tile(
    image: &mut Vec<u8>,
    image_size: (usize, usize),
    tile: &Vec<u8>,
    begin_pos: (usize, usize),
    tile_size: (usize, usize),
) {
    const NUM_CHANNELS: usize = 3;
    for ty in 0..tile_size.1 {
        for tx in 0..tile_size.0 {
            let x = begin_pos.0 + tx;
            let y = begin_pos.1 + ty;
            image[0 + NUM_CHANNELS * x + NUM_CHANNELS * image_size.0 * y] =
                tile[0 + NUM_CHANNELS * tx + NUM_CHANNELS * tile_size.0 * ty];
            image[1 + NUM_CHANNELS * x + NUM_CHANNELS * image_size.0 * y] =
                tile[1 + NUM_CHANNELS * tx + NUM_CHANNELS * tile_size.0 * ty];
            image[2 + NUM_CHANNELS * x + NUM_CHANNELS * image_size.0 * y] =
                tile[2 + NUM_CHANNELS * tx + NUM_CHANNELS * tile_size.0 * ty];
        }
    }
}

pub async fn render_task(renderer: Arc<Renderer> /* , &Scene */) -> Vec<u8> {
    const IMAGE_SIZE: (usize, usize) = (400, 300);
    const RENDER_TILE_MAX_SIZE: (usize, usize) = (33, 12);
    assert!(IMAGE_SIZE.0 >= RENDER_TILE_MAX_SIZE.0 && IMAGE_SIZE.1 >= RENDER_TILE_MAX_SIZE.1);

    // let mut res: Vec<u8> = Vec::with_capacity(IMAGE_SIZE.0 * IMAGE_SIZE.1 * 3);
    let mut render_image: Vec<u8> = Vec::from([0u8; IMAGE_SIZE.0 * IMAGE_SIZE.1 * 3]);

    // TODO: Dispatch initial tasks:
    // - Local render_tile tasks: As many as CPU cores.
    // - Remote render_tile tasks: As many as connected peers.
    let mut begin_height: usize = 0;
    while begin_height < IMAGE_SIZE.1 {
        // FIXME: Im pretty sure this is doable with better maths, but Im lazy now
        let tile_height = if (begin_height + RENDER_TILE_MAX_SIZE.1) <= IMAGE_SIZE.1 {
            RENDER_TILE_MAX_SIZE.1
        } else {
            IMAGE_SIZE.1 % RENDER_TILE_MAX_SIZE.1
        };
        let mut begin_width: usize = 0;
        while begin_width < IMAGE_SIZE.0 {
            // FIXME: Im pretty sure this is doable with better maths, but Im lazy now
            let tile_width = if (begin_width + RENDER_TILE_MAX_SIZE.0) <= IMAGE_SIZE.0 {
                RENDER_TILE_MAX_SIZE.0
            } else {
                IMAGE_SIZE.0 % RENDER_TILE_MAX_SIZE.0
            };

            // res.append(&mut renderer.render_tile((tile_width, tile_height)));
            let tile_size = (tile_width, tile_height);
            let begin_pos = (begin_width, begin_height);
            let tile = renderer.render_tile(tile_size);
            image_insert_tile(&mut render_image, IMAGE_SIZE, &tile, begin_pos, tile_size);

            begin_width += RENDER_TILE_MAX_SIZE.0;
        }
        begin_height += RENDER_TILE_MAX_SIZE.1;
    }

    debug!("Finished rendering");
    render_image
}
