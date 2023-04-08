//! Simple statistics functions.

struct SubAvg {
    avg: f64,
    count: f64,
}

// Compute the average of all the numbers in the given slice using divide and
// conquer.
fn mega_mean(slice: &[u64]) -> SubAvg {
    let len = slice.len();

    // Brute-force average 512 numbers or less.
    if len <= 512 {
        let mut avg = SubAvg {
            avg: 0.0,
            count: len as f64,
        };
        for f in slice {
            avg.avg += *f as f64;
        }
        avg.avg /= avg.count;
        return avg;
    }

    let mid = len / 2;
    let (sub_a, sub_b) = rayon::join(|| mega_mean(&slice[0..mid]), || mega_mean(&slice[mid..]));

    SubAvg {
        avg: sub_a.avg * sub_a.count / (len as f64) + sub_b.avg * sub_b.count / (len as f64),
        count: len as f64,
    }
}

// Compute the average of all the numbers in the given vector.
pub fn mean(list: &[u64]) -> f64 {
    mega_mean(list).avg
}

fn mega_stddev_sum(slice: &[u64], n: f64, mean: f64) -> f64 {
    let len = slice.len();

    if len <= 512 {
        let mut sum = 0.0;
        for x in slice {
            let tmp = *x as f64 - mean;
            sum += tmp * tmp;
        }
        return sum / n;
    }

    let mid = len / 2;
    let (sub_a, sub_b) = rayon::join(
        || mega_stddev_sum(&slice[0..mid], n, mean),
        || mega_stddev_sum(&slice[mid..], n, mean),
    );

    sub_a + sub_b
}

// Compute the standard deviation of a set of numbers determined by the
// first parameter, value of the `mean` must be the average of all the
// numbers in the same list, you can use `mean` function in this module
// to produce the required result, in case its not provided (i.e the
// value `None` was passed), this function computes the mean by itself.
pub fn stddev(list: &[u64], mean: Option<f64>) -> f64 {
    if list.len() <= 1 {
        return 0.0;
    }
    let mean = match mean {
        Some(m) => m,
        None => mega_mean(list).avg,
    };
    mega_stddev_sum(list, list.len() as f64, mean).sqrt()
}

pub fn normalized_avg(list: &mut Vec<u64>) -> f64 {
    let prev_len = list.len();
    let avg = mean(list.as_slice());
    let sd = stddev(list.as_slice(), Some(avg));
    list.retain(|&n| n as f64 >= avg - sd && n as f64 <= avg + sd);
    match list.len() {
        len if len == prev_len => avg,
        _ => mean(list),
    }
}

#[test]
fn test() {
    use rand::seq::SliceRandom;
    use rand::thread_rng;
    let mut vec: Vec<u64> = (0..1000).map(|x| (2 * x + 5) as u64).collect();
    vec.shuffle(&mut thread_rng());
    let m = mean(&vec);
    assert_eq!(m, 1004.0);
    assert_eq!(stddev(vec.as_slice(), Some(m)), (333333f64).sqrt());
}
