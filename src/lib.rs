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
use std::fs::File;
use std::io::prelude::Write;
use std::io::BufReader;
use std::io::BufRead;

#[derive(Clone, Copy)]
pub struct HeatmapConfig {
    precision: u32,
    max_memory: u32,
    max_value: u64,
    slice_duration: u64,
    num_slices: usize,
    start: u64,
}

impl HeatmapConfig {
    /// create a new HeatmapConfig with the defaults
    ///
    /// # Defaults
    /// * precision => 3
    /// * max_memory => 0 (unlimited)
    /// * max_value => 1_000_000_000 (1 second in nanoseconds)
    /// * slice_duration => 60_000_000_000 (1 minute in nanoseconds)
    /// * num_slices => 60 (1 hour of heatmap)
    /// * start => 0 (start from time 0)
    pub fn new() -> HeatmapConfig {
        HeatmapConfig {
            precision: 3,
            max_memory: 0,
            max_value: 1_000_000_000,
            slice_duration: 60_000_000_000,
            num_slices: 60,
            start: 0,
        }
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
}

#[derive(Clone, Copy)]
pub struct HeatmapCounters {
    entries_total: u64,
}

impl HeatmapCounters {
    pub fn new() -> HeatmapCounters {
        HeatmapCounters { entries_total: 0 }
    }

    pub fn clear(&mut self) {
        self.entries_total = 0;
    }
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

#[derive(Clone)]
pub struct HeatmapSlice {
    pub start: u64,
    pub stop: u64,
    pub histogram: Histogram,
}

impl HeatmapSlice {
    pub fn start(self) -> u64 {
        self.start
    }

    pub fn stop(self) -> u64 {
        self.stop
    }

    pub fn histogram(self) -> Histogram {
        self.histogram
    }
}

impl Iterator for Heatmap {
    type Item = HeatmapSlice;

    fn next(&mut self) -> Option<HeatmapSlice> {
        let current = self.data.iterator;
        
        self.data.iterator += 1;

        if self.data.iterator == (self.config.num_slices as usize) {
            self.data.iterator = 0;
            None
        } else {
            let start = (self.data.iterator as u64 * self.config.slice_duration) + self.data.start;
            Some(HeatmapSlice {
                start: start,
                stop: start + self.config.slice_duration,
                histogram: self.data.data[current].clone(),
            })
        }

        
    }
}

impl Heatmap {

    /// create a new Heatmap with defaults
    ///
    /// # Example
    /// ```
    /// # use heatmap::Heatmap;
    ///
    /// let mut h = Heatmap::new().unwrap();
    pub fn new() -> Option<Heatmap> {
        Heatmap::configured(HeatmapConfig::new())
    }

    /// create a new Heatmap
    ///
    /// # Example
    /// ```
    /// # use heatmap::{Heatmap,HeatmapConfig};
    ///
    /// let mut c = HeatmapConfig::new();
    /// c.precision(4); // set precision to 4 digits
    /// c.max_value(1_000_000_000); // store values up to 1 Million
    /// c.slice_duration(1_000_000_000); // 1 second slices
    /// c.num_slices(300); // 300 slices => 5 minutes of records
    ///
    /// let mut h = Heatmap::configured(c).unwrap();
    pub fn configured(config: HeatmapConfig) -> Option<Heatmap> {

        let mut data = Vec::new();

        for i in 0..config.num_slices {
            let mut c = HistogramConfig::new();
            c.max_value(config.max_value);
            c.precision(config.precision);
            c.max_memory(config.max_memory / config.num_slices as u32);

            data.push(Histogram::configured(c).unwrap());
        }

        let start = config.start;

        Some(Heatmap {
            config: config,
            data: HeatmapData {
                data: data,
                counters: HeatmapCounters::new(),
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
    /// let mut h = Heatmap::new().unwrap();
    ///
    /// h.increment(1, 1);
    /// assert_eq!(h.entries(), 1);
    /// h.clear();
    /// assert_eq!(h.entries(), 0);
    pub fn clear(&mut self) -> Result<(), &'static str> {
        for i in 0..self.config.num_slices {
            match self.data.data[i].clear() {
                Ok(_) => {}
                Err(e) => {
                    return Err(e);
                }
            }
        }

        self.data.counters.clear();
        self.data.start = time::precise_time_ns();
        self.data.stop = self.data.start +
                         self.config.slice_duration * self.config.num_slices as u64;
        Ok(())
    }

    /// increment the count for a value at a time
    ///
    /// # Example
    /// ```
    /// extern crate heatmap;
    /// extern crate time;
    ///
    /// let mut h = heatmap::Heatmap::new().unwrap();
    ///
    /// h.increment(time::precise_time_ns(), 1);
    /// assert_eq!(h.entries(), 1);
    pub fn increment(&mut self, time: u64, value: u64) -> Result<(), &'static str> {
        self.record(time, value, 1_u64)
    }

    /// record additional counts for value at a time
    ///
    /// # Example
    /// ```
    /// extern crate heatmap;
    /// extern crate time;
    ///
    /// let mut h = heatmap::Heatmap::new().unwrap();
    ///
    /// h.record(time::precise_time_ns(), 1, 1);
    /// assert_eq!(h.entries(), 1);
    ///
    /// h.record(time::precise_time_ns(), 2, 2);
    /// assert_eq!(h.entries(), 3);
    ///
    /// h.record(time::precise_time_ns(), 10, 10);
    /// assert_eq!(h.entries(), 13);
    pub fn record(&mut self, time: u64, value: u64, count: u64) -> Result<(), &'static str> {
        self.data.counters.entries_total = self.data.counters.entries_total.saturating_add(count);

        match self.histogram_index(time) {
            Ok(histogram_index) => {
                self.data.data[histogram_index].record(value, count)
            }
            Err(e) => {
                Err(e)
            }
        }
    }

    pub fn get(&mut self, time: u64, value: u64) -> Result<u64, &'static str> {
        match self.histogram_index(time) {
            Ok(histogram_index) => {
                match self.data.data[histogram_index].get(value) {
                    Some(count) => {
                        Ok(count)
                    }
                    None => {
                        Err("histogram didn't have")
                    }
                }
            }
            Err(e) => {
                Err(e)
            }
        }
    }

    /// internal function to find the index of the histogram in the heatmap
    fn histogram_index(&mut self, time: u64) -> Result<usize, &'static str> {
        if time < self.data.start {
            return Err("sample too early");
        } else if time > self.data.stop {
            return Err("sample too late");
        }
        let index: usize = ((time - self.data.start) as f64 /
                            self.config.slice_duration as f64)
                               .floor() as usize;
        //println!("index: time: {} is {}", time, index);
        Ok(index)
    }

    /// return the number of entries in the Histogram
    ///
    /// # Example
    /// ```
    /// extern crate heatmap;
    /// extern crate time;
    ///
    /// let mut h = heatmap::Heatmap::new().unwrap();
    ///
    /// assert_eq!(h.entries(), 0);
    /// h.record(time::precise_time_ns(), 1, 1);
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
    /// let mut c = heatmap::HeatmapConfig::new();
    /// c.num_slices(60);
    ///
    /// let mut a = heatmap::Heatmap::configured(c).unwrap();
    ///
    /// let mut b = heatmap::Heatmap::new().unwrap();
    ///
    /// assert_eq!(a.entries(), 0);
    /// assert_eq!(b.entries(), 0);
    ///
    /// let _ = a.increment(1, 1);
    /// let _ = b.increment(1, 1);
    ///
    /// assert_eq!(a.entries(), 1);
    /// assert_eq!(b.entries(), 1);
    ///
    /// a.merge(&mut b);
    ///
    /// assert_eq!(a.entries(), 2);
    /// assert_eq!(a.get(1, 1).unwrap(), 1);
    /// assert_eq!(a.get(2, 1).unwrap(), 1);
    pub fn merge(&mut self, other: &mut Heatmap) {
        loop {
            match other.next() {
                Some(other_slice) => {
                    match self.histogram_index(other_slice.clone().start()) {
                        Ok(i) => {
                            self.data.data[i].merge(&mut other_slice.clone().histogram.clone());
                            self.data.counters.entries_total += other_slice.clone()
                                                                           .histogram
                                                                           .clone()
                                                                           .entries();
                        }
                        Err(_) => {}
                    }

                }
                None => {
                    break;
                }
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
                            self.config.start).into_bytes();
        let _ = file_handle.write_all(&config);

        loop {
            match self.next() {
                Some(slice) => {
                    let mut histogram = slice.histogram.clone();
                    loop {
                        match histogram.next() {
                            Some(bucket) => {
                                if bucket.count() > 0 {
                                    let line = format!("{} {} {}\n",
                                                    slice.start,
                                                    bucket.value(),
                                                    bucket.count()).into_bytes();
                                    let _ = file_handle.write_all(&line);
                                }
                            }
                            None => { 
                                break;
                            }
                        }
                    }
                    
                }
                None => { 
                    break;
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

        let mut config = HeatmapConfig::new();
        config.precision(precision);
        config.max_memory(max_memory);
        config.max_value(max_value);
        config.slice_duration(slice_duration);
        config.num_slices(num_slices);
        config.start(start);
        let mut heatmap = Heatmap::configured(config).unwrap();

        loop {
            match lines.next() {
                Some(line) => {
                    match line {
                        Ok(s) => {
                            let tokens: Vec<&str> = s.split_whitespace().collect();
                            if tokens.len() != 3 {
                                panic!("malformed heatmap file");
                            }
                            let start: u64 = tokens[0].parse().unwrap();
                            let value: u64 = tokens[1].parse().unwrap();
                            let count: u64 = tokens[2].parse().unwrap();
                            let _ = heatmap.record(start, value, count);
                        }
                        Err(_) => { }
                    }
                }
                None => { 
                    break;
                }
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
