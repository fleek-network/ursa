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
                    .as_nanos()
            }
            let identifiers = format!($($t)*);
            println!("BENCH_START,{location},{time},{identifiers}", time = now());
            let val = { $e };
            println!("BENCH_END,{location},{time},{identifiers}", time = now());
            val
        }
        #[cfg(not(feature = "benchmarks"))]
        {
            $e
        }
    }};
}
