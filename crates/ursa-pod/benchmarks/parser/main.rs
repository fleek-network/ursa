/// Parse some instrument output into useful data.
///
/// Output is csv, with the following columns:
///  - "BENCH_BEGIN" || "BENCH_END" (prefix)
///  - location (file:line:column)
///  - timestamp (in nanoseconds)
///  - session index
use std::{
    cell::RefCell,
    io::{stdin, BufRead},
    str::FromStr,
};

use fnv::FnvHashMap;
use serde::Serialize;

mod stat;

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
            stats: Stats::compute(&mut self.inputs.borrow_mut()),
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
    pub fn compute(inputs: &mut [u64]) -> Self {
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

fn main() {
    // collect up data
    let mut filter = Filtered::default();

    let mut stdin = stdin().lock().lines();
    while let Some(Ok(line)) = stdin.next() {
        let vals: Vec<&str> = line.split(',').collect();
        let start = u64::from_str(&vals[1].replace("start=", "")).unwrap();
        let end = u64::from_str(&vals[2].replace("end=", "")).unwrap();
        let elapsed = end - start;

        let location = vals[3].to_string();
        //let id = u128::from_str(&vals[4].replace("sid=", "")).unwrap();

        let mut params = vec![location];
        let mut p2: Vec<String> = vals[4..].iter().map(|s| s.to_string()).collect();
        params.append(&mut p2);
        filter.feed(elapsed, &params);
    }

    println!("{}", serde_json::to_string_pretty(&filter).unwrap());
}
