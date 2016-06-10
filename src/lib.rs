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

use histogram::Histogram;
use std::fs::File;
use std::io::prelude::Write;
use std::io::BufReader;
use std::io::BufRead;

#[derive(Clone, Copy)]
pub struct Config {
    precision: u32,
    max_memory: u32,
    max_value: u64,
    slice_duration: u64,
    num_slices: usize,
    start: u64,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            precision: 3,
            max_memory: 0,
            max_value: 1_000_000_000,
            slice_duration: 60_000_000_000,
            num_slices: 60,
            start: 0,
        }
    }
}

impl Config {
    /// create a new Config with the defaults
    ///
    /// # Defaults
    /// * precision => 3
    /// * max_memory => 0 (unlimited)
    /// * max_value => 1_000_000_000 (1 second in nanoseconds)
    /// * slice_duration => 60_000_000_000 (1 minute in nanoseconds)
    /// * num_slices => 60 (1 hour of heatmap)
    /// * start => 0 (start from time 0)
    pub fn new() -> Config {
        Default::default()
    }

    pub fn precision(&mut self, precision: u32) -> &mut Self {
        self.precision = precision;
        self
    }

    pub fn max_memory(&mut self, bytes: u32) -> &mut Self {
        self.max_memory = bytes;
        self
    }

    pub fn max_value(&mut self, value: u64) -> &mut Self {
        self.max_value = value;
        self
    }

    pub fn slice_duration(&mut self, duration: u64) -> &mut Self {
        self.slice_duration = duration;
        self
    }

    pub fn num_slices(&mut self, count: usize) -> &mut Self {
        self.num_slices = count;
        self
    }

    pub fn start(&mut self, time: u64) -> &mut Self {
        self.start = time;
        self
    }

    pub fn build(self) -> Option<Heatmap> {
        Heatmap::configured(self)
    }
}

#[derive(Clone, Copy)]
pub struct Counters {
    entries_total: u64,
}

impl Default for Counters {
    fn default() -> Counters {
        Counters { entries_total: 0 }
    }
}

impl Counters {
    pub fn new() -> Counters {
        Default::default()
    }

    pub fn clear(&mut self) {
        self.entries_total = 0;
    }
}

#[derive(Clone)]
pub struct Data {
    data: Vec<Histogram>,
    counters: Counters,
    iterator: usize,
    start: u64,
    stop: u64,
}

#[derive(Clone, Copy)]
pub struct Properties;

#[derive(Clone)]
pub struct Heatmap {
    config: Config,
    data: Data,
    properties: Properties,
}

#[derive(Clone)]
pub struct Slice {
    pub start: u64,
    pub stop: u64,
    pub histogram: Histogram,
}

impl Slice {
    pub fn start(&self) -> u64 {
        self.start
    }

    pub fn stop(&self) -> u64 {
        self.stop
    }

    pub fn histogram(self) -> Histogram {
        self.histogram
    }
}

/// Iterator over a Heatmap's slices.
pub struct Iter<'a> {
    heatmap: &'a Heatmap,
    index: usize,
}

impl<'a> Iter<'a> {
    fn new(heatmap: &'a Heatmap) -> Iter<'a> {
        Iter {
            heatmap: heatmap,
            index: 0,
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = Slice;

    fn next(&mut self) -> Option<Slice> {
        if self.index == (self.heatmap.config.num_slices as usize) {
            None
        } else {
            let start = (self.index as u64 * self.heatmap.config.slice_duration) +
                        self.heatmap.data.start;
            let current = self.index;
            self.index += 1;
            Some(Slice {
                start: start,
                stop: start + self.heatmap.config.slice_duration,
                histogram: self.heatmap.data.data[current].clone(),
            })
        }
    }
}

impl<'a> IntoIterator for &'a Heatmap {
    type Item = Slice;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Iter::new(self)
    }
}

impl Default for Heatmap {
    fn default() -> Heatmap {
        Heatmap::configured(Config::new()).unwrap()
    }
}

impl Heatmap {
    /// create a new Heatmap with defaults
    ///
    /// # Example
    /// ```
    /// # use heatmap::Heatmap;
    ///
    /// let mut h = Heatmap::new();
    pub fn new() -> Heatmap {
        Default::default()
    }

    /// configure and build a new Heatmap
    ///
    /// # Example
    /// ```
    /// # use heatmap::Heatmap;
    ///
    /// let mut heatmap = Heatmap::configure()
    ///     .precision(4) // set precision to 4 digits
    ///     .max_value(1_000_000_000) // store values up to 1 Million
    ///     .slice_duration(1_000_000_000) // 1 second slices
    ///     .num_slices(300) // 300 slices => 5 minutes of records
    ///     .build() // create the Heatmap
    ///     .unwrap();
    pub fn configure() -> Config {
        Config::default()
    }

    fn configured(config: Config) -> Option<Heatmap> {
        let mut data = Vec::new();

        for _ in 0..config.num_slices {
            data.push(Histogram::configure()
                .max_value(config.max_value)
                .precision(config.precision)
                .max_memory(config.max_memory / config.num_slices as u32)
                .build()
                .unwrap()
                );
        }

        let start = config.start;

        Some(Heatmap {
            config: config,
            data: Data {
                data: data,
                counters: Counters::new(),
                iterator: 0,
                start: start,
                stop: start + config.slice_duration * config.num_slices as u64,
            },
            properties: Properties,
        })
    }

    /// clear the heatmap data
    ///
    /// # Example
    /// ```
    /// # use heatmap::Heatmap;
    ///
    /// let mut h = Heatmap::new();
    ///
    /// h.increment(1, 1);
    /// assert_eq!(h.entries(), 1);
    /// h.clear();
    /// assert_eq!(h.entries(), 0);
    pub fn clear(&mut self) {
        for i in 0..self.config.num_slices {
            self.data.data[i].clear();
        }

        self.data.counters.clear();
        self.data.start = time::precise_time_ns();
        self.data.stop = self.data.start +
                         self.config.slice_duration * self.config.num_slices as u64;
    }

    /// increment the count for a value at a time
    ///
    /// # Example
    /// ```
    /// extern crate heatmap;
    /// extern crate time;
    ///
    /// let mut h = heatmap::Heatmap::new();
    ///
    /// h.increment(time::precise_time_ns(), 1);
    /// assert_eq!(h.entries(), 1);
    pub fn increment(&mut self, time: u64, value: u64) -> Result<(), &'static str> {
        self.increment_by(time, value, 1_u64)
    }

    /// increment additional counts for value at a time
    ///
    /// # Example
    /// ```
    /// extern crate heatmap;
    /// extern crate time;
    ///
    /// let mut h = heatmap::Heatmap::new();
    ///
    /// h.increment_by(time::precise_time_ns(), 1, 1);
    /// assert_eq!(h.entries(), 1);
    ///
    /// h.increment_by(time::precise_time_ns(), 2, 2);
    /// assert_eq!(h.entries(), 3);
    ///
    /// h.increment_by(time::precise_time_ns(), 10, 10);
    /// assert_eq!(h.entries(), 13);
    pub fn increment_by(&mut self, time: u64, value: u64, count: u64) -> Result<(), &'static str> {
        self.data.counters.entries_total = self.data.counters.entries_total.saturating_add(count);

        match self.histogram_index(time) {
            Ok(histogram_index) => self.data.data[histogram_index].increment_by(value, count),
            Err(e) => Err(e),
        }
    }

    pub fn get(&mut self, time: u64, value: u64) -> Result<u64, &'static str> {
        match self.histogram_index(time) {
            Ok(histogram_index) => {
                match self.data.data[histogram_index].get(value) {
                    Some(count) => Ok(count),
                    None => Err("histogram didn't have"),
                }
            }
            Err(e) => Err(e),
        }
    }

    /// internal function to find the index of the histogram in the heatmap
    fn histogram_index(&mut self, time: u64) -> Result<usize, &'static str> {
        if time < self.data.start {
            return Err("sample too early");
        } else if time > self.data.stop {
            return Err("sample too late");
        }
        let index: usize =
            ((time - self.data.start) as f64 / self.config.slice_duration as f64).floor() as usize;
        Ok(index)
    }

    /// return the number of entries in the Histogram
    ///
    /// # Example
    /// ```
    /// extern crate heatmap;
    /// extern crate time;
    ///
    /// let mut h = heatmap::Heatmap::new();
    ///
    /// assert_eq!(h.entries(), 0);
    /// h.increment_by(time::precise_time_ns(), 1, 1);
    /// assert_eq!(h.entries(), 1);
    pub fn entries(&mut self) -> u64 {
        self.data.counters.entries_total
    }

    /// merge one Heatmap into another Heatmap
    ///
    /// # Example
    /// ```
    /// extern crate heatmap;
    /// extern crate time;
    ///
    /// let mut a = heatmap::Heatmap::configure()
    ///     .num_slices(60)
    ///     .slice_duration(1)
    ///     .build()
    ///     .unwrap();
    ///
    /// let mut b = heatmap::Heatmap::new();
    ///
    /// assert_eq!(a.entries(), 0);
    /// assert_eq!(b.entries(), 0);
    ///
    /// let _ = a.increment(0, 1);
    /// let _ = b.increment(0, 1);
    ///
    /// assert_eq!(a.entries(), 1);
    /// assert_eq!(b.entries(), 1);
    ///
    /// a.merge(&mut b);
    ///
    /// assert_eq!(a.entries(), 2);
    /// assert_eq!(a.get(0, 1).unwrap(), 2);
    /// assert_eq!(a.get(0, 2).unwrap(), 0);
    /// assert_eq!(a.get(1, 1).unwrap(), 0);
    pub fn merge(&mut self, other: &mut Heatmap) {
        for slice in other.into_iter() {
            let slice = slice.clone();
            let start = slice.start();
            for bucket in &slice.histogram {
                if bucket.count() > 0 {
                    println!("start: {} bucket: {} count: {}",
                             start,
                             bucket.value(),
                             bucket.count());
                }

                let _ = self.increment_by(start, bucket.value(), bucket.count());
            }
        }
    }

    pub fn save(&mut self, file: String) {
        let mut file_handle = File::create(file.clone()).unwrap();

        let config = format!("{} {} {} {} {} {}\n",
                             self.config.precision,
                             self.config.max_memory,
                             self.config.max_value,
                             self.config.slice_duration,
                             self.config.num_slices,
                             self.config.start)
            .into_bytes();
        let _ = file_handle.write_all(&config);

        for slice in self.into_iter() {
            let histogram = slice.histogram.clone();
            for bucket in &histogram {
                if bucket.count() > 0 {
                    let line = format!("{} {} {}\n", slice.start, bucket.value(), bucket.count())
                        .into_bytes();
                    let _ = file_handle.write_all(&line);
                }
            }
        }

    }

    pub fn load(file: String) -> Heatmap {
        let file_handle = File::open(file.clone()).unwrap();

        let reader = BufReader::new(&file_handle);

        let mut lines = reader.lines();

        let config = lines.next().unwrap().unwrap();
        let config_tokens: Vec<&str> = config.split_whitespace().collect();

        let precision: u32 = config_tokens[0].parse().unwrap();
        let max_memory: u32 = config_tokens[1].parse().unwrap();
        let max_value: u64 = config_tokens[2].parse().unwrap();
        let slice_duration: u64 = config_tokens[3].parse().unwrap();
        let num_slices: usize = config_tokens[4].parse().unwrap();
        let start: u64 = config_tokens[5].parse().unwrap();

        let mut config = Config::new();
        config.precision(precision);
        config.max_memory(max_memory);
        config.max_value(max_value);
        config.slice_duration(slice_duration);
        config.num_slices(num_slices);
        config.start(start);
        let mut heatmap = Heatmap::configured(config).unwrap();

        for line in lines {
            if let Ok(s) = line {
                let tokens: Vec<&str> = s.split_whitespace().collect();
                if tokens.len() != 3 {
                    panic!("malformed heatmap file");
                }
                let start: u64 = tokens[0].parse().unwrap();
                let value: u64 = tokens[1].parse().unwrap();
                let count: u64 = tokens[2].parse().unwrap();
                let _ = heatmap.increment_by(start, value, count);
            }
        }

        heatmap
    }

    pub fn histogram_buckets(&self) -> u64 {
        self.data.data[0].clone().buckets_total()
    }

    pub fn num_slices(&self) -> u64 {
        self.config.num_slices as u64
    }
}
