/// Parse some instrument output into useful data.
///
/// Output is csv, with the following columns:
///  - "BENCH_BEGIN" || "BENCH_END" (prefix)
///  - location (file:line:column)
///  - timestamp (in nanoseconds)
///  - session index
use std::{
    io::{stdin, BufRead},
    str::FromStr,
};

use crate::types::Filtered;

mod stat;
mod types;

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
