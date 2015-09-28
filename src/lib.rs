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

#[derive(Clone, Copy, Default)]
pub struct HeatmapCounters {
    entries_total: u64,
}

#[derive(Clone)]
pub struct HeatmapData {
    data: Vec<Histogram>,
    counters: HeatmapCounters,
    iterator: usize,
    start: u64,
    stop: u64,
}

#[derive(Clone, Copy)]
pub struct HeatmapProperties;

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
            data[i] = Histogram::new(HistogramConfig {
                max_value: config.max_value,
                precision: config.precision,
                max_memory: config.max_memory,

            }).unwrap();
        }

        let start = time::precise_time_ns();

        Some(Heatmap {
            config: config,
            data: HeatmapData {
                data: data,
                counters: HeatmapCounters {
                    entries_total: 0,
                },
                iterator: 0,
                start: start,
                stop: start + config.slice_duration * config.num_slices as u64,
            },
            properties: HeatmapProperties,
        })
    }

    /// clear the heatmap data
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
    ///
    /// h.increment(1);
    /// assert_eq!(h.entries(), 1);
    /// h.clear();
    /// assert_eq!(h.entries(), 0);
    pub fn clear(&mut self) {
        for i in 0..self.config.num_slices {
            self.data.data[i].clear();
        }

        self.data.counters = Default::default();
        self.data.start = time::precise_time_ns();
        self.data.stop =
            self.data.start + self.config.slice_duration * self.config.num_slices as u64;
    }

    /// increment the count for a value at a time
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
    ///
    /// h.increment(time::precise_time_ns(), 1);
    /// assert_eq!(h.entries(), 1);
    pub fn increment(&mut self, time: u64, value: u64) {
        self.record(time, value, 1_u64);
    }

    /// record additional counts for value at a time
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
    ///
    /// h.record(time::precise_time_ns(), 1, 1);
    /// assert_eq!(h.entries(), 1);
    ///
    /// h.record(time::precise_time_ns(), 2, 2);
    /// assert_eq!(h.entries(), 3);
    ///
    /// h.record(time::precise_time_ns(), 10, 10);
    /// assert_eq!(h.entries(), 13);
    pub fn record(&mut self, time: u64, value: u64, count: u64) {
        self.data.counters.entries_total = self.data.counters.entries_total.saturating_add(count);

        let histogram_index =
            ((time - self.data.start) as f64 / self.config.slice_duration as f64).floor() as usize;

        self.data.data[histogram_index].record(value, count);
    }

    /// return the number of entries in the Histogram
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
    ///
    /// assert_eq!(h.entries(), 0);
    /// h.record(time::precise_time_ns(), 1, 1);
    /// assert_eq!(h.entries(), 1);
    pub fn entries(&mut self) -> u64 {
        self.data.counters.entries_total
    }
}

#[test]
fn it_works() {
}
