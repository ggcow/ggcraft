#![allow(unused)]
use std::{
    borrow::Cow,
    fs::{self, File},
};

use fastanvil::Region;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

const WORLD_REGIONS: &'static str = "../hermitcraft10/region";

pub struct McLoader<'a> {
    regions: FxHashMap<(i32, i32), Option<Region<File>>>,
    chunks: FxHashMap<(i32, i32), Option<CurrentJavaChunk<'a>>>,
}

impl McLoader<'_> {
    pub fn new() -> Self {
        Self {
            regions: Default::default(),
            chunks: Default::default(),
        }
    }

    fn get_chunklet(&mut self, (cx, cy, cz): (i32, i32, i32)) -> Option<&BlockStates<'_>> {
        let chunk = self.get_chunk((cx, cz))?;

        let chunklet = &chunk
            .sections
            .iter()
            .find(|s| s.y == cy as i8)?
            .block_states;

        Some(chunklet)
    }

    fn get_chunk(&mut self, (cx, cz): (i32, i32)) -> Option<&CurrentJavaChunk<'_>> {
        let rx = cx >> 5;
        let rz = cz >> 5;
        let mut region = self.regions.entry((rx, rz)).or_insert_with(|| {
            let fname = format!("{WORLD_REGIONS}/r.{rx}.{rz}.mca");
            println!("loading region {rx}.{rz}");
            let region_file = fs::File::open(fname).ok()?;

            let mut region = fastanvil::Region::from_stream(region_file).ok()?;

            Some(region)
        });

        let Some(region) = region else { return None };

        self.chunks
            .entry((cx, cz))
            .or_insert_with(|| {
                // println!("loading chunk {cx}.{cz}");
                let chunk = region
                    .read_chunk(cx.rem_euclid(32) as usize, cz.rem_euclid(32) as usize)
                    .ok()??;

                let chunk: Option<CurrentJavaChunk> = fastnbt::from_bytes(&chunk).ok();
                chunk
            })
            .as_ref()
    }

    /// get name from world pos
    pub fn get_block_name(&mut self, pos: [i32; 3]) -> Option<String> {
        let cx = pos[0] >> 4;
        let cz = pos[2] >> 4;

        let chunk = self.get_chunk((cx, cz))?;
        let chunklet_y = (pos[1] >> 5) as i8;

        let chunklet = &chunk
            .sections
            .iter()
            .find(|s| s.y == chunklet_y)
            .unwrap()
            .block_states;

        let Some(indices) = &chunklet.data else {
            return Some(chunklet.palette[0].name.to_string());
        };

        let len_log2 = usize::BITS - (chunklet.palette.len() - 1).leading_zeros();

        let bits_per_index = (len_log2 as usize).max(4);
        let how_many_packed = 64 / bits_per_index;

        // unpacked index (0..4096)
        let idx = ((pos[1] & 0xf) * 16 * 16 + (pos[2] & 0xf) * 16 + (pos[0] & 0xf)) as usize;

        let packed_i64 = idx / how_many_packed;
        let shift = idx % how_many_packed;

        // println!(
        //     "len: {}, bits {bits_per_index} packing {how_many_packed}",
        //     chunklet.palette.len()
        // );
        // println!("{pos:?} {idx} {packed_i64} {shift}");
        let index = indices[packed_i64] >> (shift * bits_per_index);

        let block = &chunklet.palette[(index & ((1 << bits_per_index) - 1)) as usize];

        // println!("palette: {} ({bits_per_index})", chunklet.palette.len());
        // std::fs::write(
        //     "/tmp/palette.jsonp",
        //     serde_json::to_vec_pretty(&chunklet.palette).unwrap(),
        // );

        Some(block.name.to_string())
    }
}

#[test]
fn test() {
    let mut loader = McLoader::new();
    println!("{:?}", loader.get_block_name([-10, 0, -10]));
    // println!("{:?}", loader.get_block_name([0, 0, 0]));
    // println!("{:?}", loader.get_block_name([1, 0, 0]));
    // println!("{:?}", loader.get_block_name([2, 0, 0]));
    // println!("{:?}", loader.get_block_name([3, 0, 0]));
    // println!("{:?}", loader.get_block_name([7, 0, 0]));
    // println!("{:?}", loader.get_block_name([8, 0, 0]));
    // println!("{:?}", loader.get_block_name([9, 0, 0]));

    // println!("{:?}", loader.get_block_name([0, 1, 0]));
    // println!("{:?}", loader.get_block_name([0, 0, 1]));
    // println!("{:?}", loader.get_block_name([15, 15, 15]));
}

#[derive(Deserialize, Serialize, Debug)]
struct CurrentJavaChunk<'a> {
    #[serde(rename = "Status")]
    status: String,
    sections: Vec<ChunksSection<'a>>,
    #[serde(rename = "xPos")]
    x_pos: i32,
    #[serde(rename = "yPos")]
    y_pos: i32,
    #[serde(rename = "zPos")]
    z_pos: i32,
}

#[derive(Deserialize, Serialize, Debug)]
struct ChunksSection<'a> {
    #[serde(rename = "Y")]
    y: i8,

    block_states: BlockStates<'a>,
}

#[derive(Deserialize, Serialize, Debug)]
struct BlockStates<'a> {
    palette: Vec<BlockPalette<'a>>,
    data: Option<fastnbt::LongArray>, // None = palette.len() == 1
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
struct BlockPalette<'a> {
    #[serde(rename = "Name")]
    name: Cow<'a, str>,
    #[serde(
        rename = "Properties",
        skip_serializing_if = "Option::is_none",
        default
    )]
    // value will only be bool, int or string
    // probably should intern strings here as well
    properties: Option<FxHashMap<String, fastnbt::Value>>,
}
