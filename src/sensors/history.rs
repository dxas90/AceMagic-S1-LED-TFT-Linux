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
