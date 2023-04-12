use std::cell::RefCell;

use fnv::FnvHashMap;
use rayon::prelude::*;
use serde::Serialize;

use crate::stat;

#[derive(Default)]
pub struct Filtered {
    inputs: RefCell<Vec<u64>>,
    parameters: FnvHashMap<String, Filtered>,
}

impl Filtered {
    pub fn feed(&mut self, input: u64, parameters: &[String]) {
        self.inputs.borrow_mut().push(input);
        if !parameters.is_empty() {
            for (i, param) in parameters[1..].iter().enumerate() {
                self.parameters
                    .entry(param.clone())
                    .or_default()
                    .feed(input, &parameters[i + 1..]);
            }
        }
    }
}

impl Serialize for Filtered {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct FilteredAggr<'a> {
            stats: Stats,
            parameters: &'a FnvHashMap<String, Filtered>,
        }

        let aggr = FilteredAggr {
            stats: Stats::new(&mut self.inputs.borrow_mut()),
            parameters: &self.parameters,
        };

        aggr.serialize(serializer)
    }
}

#[derive(Serialize)]
struct Stats {
    count: usize,
    mean: f64,
    median: f64,
    std_dev: f64,
    sum: u128,
}

impl Stats {
    pub fn new(inputs: &mut [u64]) -> Self {
        let count = inputs.len();

        inputs.par_sort();
        let median = if count % 2 != 0 {
            inputs[(count + 1) / 2] as f64
        } else {
            let index = count / 2;
            let (v1, v2) = (inputs[index] as f64, inputs[index + 1] as f64);
            (v1 + v2) / 2.0
        };

        let mean = stat::mean(&*inputs);
        let std_dev = stat::stddev(&*inputs, Some(mean));
        let sum: u128 = inputs.par_iter().map(|n| *n as u128).sum();

        Self {
            count,
            median,
            mean,
            std_dev,
            sum,
        }
    }
}
