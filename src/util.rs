use evdev_rs::TimeVal;


pub trait ElapsedSince {
    /// Calculate time elapsed since `other`.
    ///
    /// Assumes that self >= other
    fn elapsed_since(&self, other: Self) -> Self;
}

impl ElapsedSince for TimeVal {
    // useful when we need sub-second hold durations. currently unused.
    fn elapsed_since(&self, other: Self) -> Self {
        const USEC_PER_SEC: u32 = 1_000_000;
        let (secs, nsec) = if self.tv_usec >= other.tv_usec {
            (
                (self.tv_sec - other.tv_sec) as i64,
                (self.tv_usec - other.tv_usec) as u32,
            )
        } else {
            (
                (self.tv_sec - other.tv_sec - 1) as i64,
                self.tv_usec as u32 + (USEC_PER_SEC as u32) - other.tv_usec as u32,
            )
        };

        Self {
            tv_sec: secs,
            tv_usec: nsec as i64,
        }
    }
}
