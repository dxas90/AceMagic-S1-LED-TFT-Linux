use std::collections::VecDeque;

pub struct History {
    buf: VecDeque<f64>,
    max: usize,
}

impl History {
    pub fn new(max: usize) -> Self {
        let max = max.max(1);
        let mut buf = VecDeque::with_capacity(max);
        for _ in 0..max {
            buf.push_back(0.0);
        }
        Self { buf, max }
    }

    pub fn push(&mut self, v: f64) {
        self.buf.push_back(v);
        if self.buf.len() > self.max {
            self.buf.pop_front();
        }
    }

    pub fn last(&self) -> f64 {
        *self.buf.back().unwrap_or(&0.0)
    }

    /// Format as comma-separated values for use in {N} history tokens.
    pub fn join_csv(&self) -> String {
        self.buf
            .iter()
            .map(|v| format!("{:.1}", v))
            .collect::<Vec<_>>()
            .join(",")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_fills_with_zeros() {
        let h = History::new(4);
        assert_eq!(h.join_csv(), "0.0,0.0,0.0,0.0");
    }

    #[test]
    fn push_replaces_oldest() {
        let mut h = History::new(3);
        h.push(1.0);
        h.push(2.0);
        h.push(3.0);
        h.push(4.0); // should evict oldest 0.0 then oldest 1.0? No — starts with 3 zeros.
                     // After new(3): [0,0,0]. push(1)→[0,0,1], push(2)→[0,1,2], push(3)→[1,2,3], push(4)→[2,3,4]
        assert_eq!(h.join_csv(), "2.0,3.0,4.0");
    }

    #[test]
    fn last_returns_most_recent() {
        let mut h = History::new(5);
        h.push(42.0);
        assert_eq!(h.last(), 42.0);
    }

    #[test]
    fn length_never_exceeds_max() {
        let mut h = History::new(4);
        for i in 0..20 {
            h.push(i as f64);
        }
        let csv = h.join_csv();
        let parts: Vec<_> = csv.split(',').collect();
        assert_eq!(parts.len(), 4);
    }

    #[test]
    fn join_csv_format() {
        let mut h = History::new(2);
        h.push(1.5);
        h.push(2.75);
        // new(2) fills [0,0], push(1.5)→[0,1.5], push(2.75)→[1.5,2.75]
        assert_eq!(h.join_csv(), "1.5,2.8");
    }

    #[test]
    fn min_capacity_one() {
        let mut h = History::new(0); // clamped to 1
        h.push(99.0);
        assert_eq!(h.last(), 99.0);
        let csv = h.join_csv();
        let parts: Vec<_> = csv.split(',').collect();
        assert_eq!(parts.len(), 1);
    }
}
