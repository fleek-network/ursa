/// Basic timing instrument. Any additional user data should be in a csv format.
#[macro_export]
macro_rules! instrument {
    ($e:expr, $($t:tt)+) => {{
        #[cfg(feature = "benchmarks")]
        {
            let location = format!("{}:{}:{}", file!(), line!(), column!());
            #[inline(always)]
            fn now() -> u128 {
                std::time::SystemTime::now()
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_micros()
            }
            let identifiers = format!($($t)*);
            let start = now();
            let val = { $e };
            println!("SAMPLE,start={start},end={end},uid={location},{identifiers}", end = now());
            val
        }
        #[cfg(not(feature = "benchmarks"))]
        {
            $e
        }
    }};
}
