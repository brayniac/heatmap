//! Heatmap provides a time-series of Histograms, which is useful for
//! recording distributions over time and reporting percentiles over time
//!
//!
//! # Goals
//! * pre-allocated datastructure
//! * report time-series percentiles
//! * auto-slicing by record time
//!
//! # Future work
//! * make it work
//! * make it awesome
//! * add roll-up
//!
//! # Usage
//! Create a heatmap. Insert values over time. Profit.
//!
//! ```
//!
//! use heatmap::*;

#![crate_type = "lib"]

#![crate_name = "heatmap"]

extern crate histogram;
extern crate time;

use histogram::*;
use time::*;

#[derive(Clone, Copy, Default)]
pub struct HeatmapConfig {
    pub precision: u32,
    pub max_memory: u32,
    pub max_value: u64,
    pub slice_duration: u64,
    pub num_slices: usize,
}

#[derive(Clone)]
pub struct HeatmapData {
    data: Vec<Histogram>,
    iterator: usize,
}

#[derive(Clone, Copy)]
pub struct HeatmapProperties {
	started: u64,
}

#[derive(Clone)]
pub struct Heatmap {
    config: HeatmapConfig,
    data: HeatmapData,
    properties: HeatmapProperties,
}

impl Heatmap {

	/// create a new Heatmap
    ///
    /// # Example
    /// ```
    /// # use heatmap::{Heatmap,HeatmapConfig};
    ///
    /// let mut h = Heatmap::new(
    ///     HeatmapConfig{
    ///         max_value: 1000000,
    ///         precision: 3,
    ///         max_memory: 0,
    ///			slice_duration: 1000000000,
    ///			num_slices: 300,
    /// }).unwrap();
	pub fn new(config: HeatmapConfig) -> Option<Heatmap> {

		let mut data = Vec::with_capacity(config.num_slices);

		unsafe {
            data.set_len(config.num_slices);
        }

        for i in 0..config.num_slices {
        	data[i] = Histogram::new(
        		HistogramConfig {
        			max_value: config.max_value,
    		        precision: config.precision,
   			        max_memory: config.max_memory,

        	}).unwrap();
        }

        Some(Heatmap {
        	config: config,
        	data: HeatmapData {
        		data: data,
        		iterator: 0,
        	},
        	properties: HeatmapProperties {
        		started: time::precise_time_ns(),
        	}
        })
	}
}

#[test]
fn it_works() {
}
