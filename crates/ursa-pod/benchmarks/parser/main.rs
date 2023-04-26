/// Parse some instrument output into useful data.
///
/// Output is csv, with the following columns:
///  - "BENCH_BEGIN" || "BENCH_END" (prefix)
///  - location (file:line:column)
///  - timestamp (in nanoseconds)
///  - session index
use std::{
    cell::RefCell,
    fs::File,
    io::{stdin, BufRead},
    path::PathBuf,
    str::FromStr,
};

use clap::{Parser, Subcommand};
use fnv::FnvHashMap;
use gnuplot::{AxesCommon, Caption, Color, Figure};
use serde::{Deserialize, Serialize};

mod stat;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Subommand to run
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse raw data from stdin
    Parse,
    /// Use gnuplot4 to render some data
    Plot {
        /// lists test values
        #[arg(short, long)]
        input: Option<PathBuf>,
        /// param for x axis, values should be able to be parsed into a valid f64
        param: String,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Parse {} => parse_raw_data(),
        Commands::Plot { input, param } => plot(input, param),
    }
}

#[derive(Default, Deserialize, Clone)]
pub struct Filtered {
    inputs: RefCell<Vec<u64>>,
    params: FnvHashMap<String, FnvHashMap<String, Filtered>>,
}

impl Filtered {
    pub fn feed(&mut self, input: u64, parameters: &[(String, String)]) {
        self.inputs.borrow_mut().push(input);
        if !parameters.is_empty() {
            for (i, (param, value)) in parameters[1..].iter().enumerate() {
                self.params
                    .entry(param.clone())
                    .or_default()
                    .entry(value.clone())
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
            params: &'a FnvHashMap<String, FnvHashMap<String, Filtered>>,
        }

        let aggr = FilteredAggr {
            stats: Stats::compute(&mut self.inputs.borrow_mut()),
            params: &self.params,
        };

        aggr.serialize(serializer)
    }
}

#[derive(Serialize, Deserialize)]
struct ParsedData {
    stats: Stats,
    params: FnvHashMap<String, FnvHashMap<String, ParsedData>>,
}

#[derive(Serialize, Deserialize)]
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

fn parse_raw_data() {
    // collect up data
    let mut filter = Filtered::default();

    let mut stdin = stdin().lock().lines();
    while let Some(Ok(line)) = stdin.next() {
        let vals: Vec<&str> = line.split(',').collect();
        let start = u64::from_str(&vals[1].replace("start=", "")).unwrap();
        let end = u64::from_str(&vals[2].replace("end=", "")).unwrap();
        let elapsed = end - start;

        let mut params = vec![];
        let mut p = vals[2..]
            .iter()
            .map(|s| {
                let v: Vec<String> = s.split('=').map(|s| s.to_string()).collect();
                if v.len() != 2 {
                    (String::new(), v[0].clone())
                } else {
                    (v[0].clone(), v[1].clone())
                }
            })
            .collect();
        params.append(&mut p);
        filter.feed(elapsed, &params);
    }

    println!("{}", serde_json::to_string_pretty(&filter).unwrap());
}

fn plot(path: Option<PathBuf>, param: String) {
    let data: ParsedData = match path {
        Some(path) => {
            let file = File::open(path).expect("open file");
            serde_json::from_reader(file).expect("parse json input from file")
        }
        None => {
            let stdin = stdin().lock();
            serde_json::from_reader(stdin).expect("parse json input from stdin")
        }
    };

    let column = data.params.get(&param).expect("invalid parameter");
    let (mut x, mut y_mean, mut y_median, mut y_std_dev) = (vec![], vec![], vec![], vec![]);
    for (
        display,
        ParsedData {
            stats:
                Stats {
                    mean,
                    median,
                    std_dev,
                    ..
                },
            ..
        },
    ) in column
    {
        x.push(u64::from_str(display).unwrap());
        y_mean.push(*mean);
        y_median.push(*median);
        y_std_dev.push(*std_dev);
    }

    let mut fg = Figure::new();
    fg.axes2d()
        .set_x_label(&param.replace('_', " "), &[])
        .set_y_label("elapsed (μs)", &[])
        .lines(&x.clone(), y_mean, &[Caption("Mean"), Color("black")])
        .lines(&x, y_median, &[Caption("Median"), Color("red")])
        .lines(&x, y_std_dev, &[Caption("Std. Deviation"), Color("blue")]);
    fg.show().unwrap();
}
