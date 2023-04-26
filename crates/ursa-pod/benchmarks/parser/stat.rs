//! Simple statistics functions.

use rayon::prelude::*;

/// Compute the median value. Sorts the list in place
pub fn median(list: &mut [u64]) -> f64 {
    let count = list.len();
    list.par_sort();
    if count % 2 != 0 {
        list[count / 2] as f64
    } else {
        let index = count / 2;
        (list[index - 1] + list[index]) as f64 / 2.0
    }
}

/// Compute the sum and mean values
pub fn sum_mean(list: &[u64]) -> (u128, f64) {
    let sum = list.par_iter().map(|n| *n as u128).sum();
    // todo: avoid the overflow
    let mean = sum as f64 / list.len() as f64;

    (sum, mean)
}

/// Compute the normalized average
pub fn _normalized_avg(list: &mut Vec<u64>, mean: Option<f64>) -> f64 {
    let prev_len = list.len();
    let mean = mean.unwrap_or_else(|| sum_mean(list).1);
    let sd = stddev(list.as_slice(), Some(mean));
    list.retain(|&n| n as f64 >= mean - sd && n as f64 <= mean + sd);
    match list.len() {
        len if len == prev_len => mean,
        _ => sum_mean(list).1,
    }
}

fn mega_stddev_sum(list: &[u64], n: f64, mean: f64) -> f64 {
    let len = list.len();

    if len <= 512 {
        let mut sum = 0.0;
        for x in list {
            let tmp = *x as f64 - mean;
            sum += tmp * tmp;
        }
        return sum / n;
    }

    let mid = len / 2;
    let (sub_a, sub_b) = rayon::join(
        || mega_stddev_sum(&list[0..mid], n, mean),
        || mega_stddev_sum(&list[mid..], n, mean),
    );

    sub_a + sub_b
}

/// Compute the standard deviation of a set of numbers determined by the
/// first parameter, value of the `mean` must be the average of all the
/// numbers in the same list, you can use `mean` function in this module
/// to produce the required result, in case its not provided (i.e the
/// value `None` was passed), this function computes the mean by itself.
pub fn stddev(list: &[u64], mean: Option<f64>) -> f64 {
    if list.len() <= 1 {
        return 0.0;
    }
    let mean = mean.unwrap_or_else(|| sum_mean(list).1);
    mega_stddev_sum(list, list.len() as f64, mean).sqrt()
}

#[test]
fn test() {
    use rand::seq::SliceRandom;
    use rand::thread_rng;
    let mut vec: Vec<u64> = (0..1000).map(|x| (2 * x + 5) as u64).collect();
    vec.shuffle(&mut thread_rng());
    let m = sum_mean(&vec).1;
    assert_eq!(m, 1004.0);
    assert_eq!(stddev(vec.as_slice(), Some(m)), (333333f64).sqrt());
}
