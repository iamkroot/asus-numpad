use evdev_rs::TimeVal;

pub(crate) trait ElapsedSince {
    /// Calculate time elapsed since `other`.
    ///
    /// Requires that self >= other
    fn elapsed_since(&self, other: Self) -> CustomDuration;
}

/// A custom struct to hold the duration in terms of microseconds
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub(crate) struct CustomDuration {
    micros: u64,
}

impl CustomDuration {
    pub(crate) const fn from_millis(millis: u64) -> Self {
        Self {
            micros: millis * 1000,
        }
    }
}

impl PartialOrd for CustomDuration {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CustomDuration {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.micros.cmp(&other.micros)
    }
}

impl ElapsedSince for TimeVal {
    fn elapsed_since(&self, other: Self) -> CustomDuration {
        debug_assert!(*self >= other, "Called on older timeval!");
        const MICROS_PER_SEC: i64 = 1_000_000;
        let (secs, usec) = if self.tv_usec >= other.tv_usec {
            ((self.tv_sec - other.tv_sec), (self.tv_usec - other.tv_usec))
        } else {
            (
                (self.tv_sec - other.tv_sec - 1),
                self.tv_usec - other.tv_usec + MICROS_PER_SEC,
            )
        };

        CustomDuration {
            micros: (secs * MICROS_PER_SEC + usec) as u64,
        }
    }
}

#[test]
fn test_elapsed_since() {
    let t1 = TimeVal {
        tv_sec: 100,
        tv_usec: 200,
    };
    let t2 = TimeVal {
        tv_sec: 101,
        tv_usec: 100,
    };
    assert_eq!(t2.elapsed_since(t1), CustomDuration { micros: 999_900 });
}
