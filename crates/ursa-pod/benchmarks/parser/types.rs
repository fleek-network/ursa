use std::cell::RefCell;

use fnv::FnvHashMap;
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
            params: &'a FnvHashMap<String, Filtered>,
        }

        let aggr = FilteredAggr {
            stats: Stats::new(&mut self.inputs.borrow_mut()),
            params: &self.parameters,
        };

        aggr.serialize(serializer)
    }
}

#[derive(Serialize)]
struct Stats {
    count: usize,
    sum: u128,
    mean: f64,
    median: f64,
    std_dev: f64,
}

impl Stats {
    pub fn new(inputs: &mut [u64]) -> Self {
        let count = inputs.len();
        let (sum, mean) = stat::sum_mean(&*inputs);
        let std_dev = stat::stddev(&*inputs, Some(mean));
        let median = stat::median(inputs);

        Self {
            count,
            median,
            mean,
            std_dev,
            sum,
        }
    }
}
