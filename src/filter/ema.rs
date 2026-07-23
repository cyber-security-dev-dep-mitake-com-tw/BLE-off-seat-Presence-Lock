/// Exponential Moving Average filter for RSSI smoothing.
///
/// `S_t = α · Y_t + (1 - α) · S_{t-1}`
#[derive(Debug, Clone)]
pub struct EmaFilter {
    alpha: f64,
    value: Option<f64>,
}

impl EmaFilter {
    /// Create a new EMA filter.
    ///
    /// # Panics
    /// Panics if `alpha` is outside \[0.0, 1.0\].
    pub fn new(alpha: f64) -> Self {
        assert!(
            (0.0..=1.0).contains(&alpha),
            "ema alpha must be in [0.0, 1.0]"
        );
        Self {
            alpha,
            value: None,
        }
    }

    /// Feed a raw sample and return the smoothed value.
    pub fn update(&mut self, sample: f64) -> f64 {
        let next = match self.value {
            Some(prev) => self.alpha * sample + (1.0 - self.alpha) * prev,
            None => sample,
        };
        self.value = Some(next);
        next
    }

    /// Current smoothed value, if any sample has been seen.
    #[allow(dead_code)]
    pub fn current(&self) -> Option<f64> {
        self.value
    }

    /// Reset filter state.
    pub fn reset(&mut self) {
        self.value = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_sample_equals_input() {
        let mut ema = EmaFilter::new(0.2);
        assert!((ema.update(-70.0) + 70.0).abs() < f64::EPSILON);
    }

    #[test]
    fn smooths_toward_new_samples() {
        let mut ema = EmaFilter::new(0.5);
        ema.update(-60.0);
        let v = ema.update(-80.0);
        // 0.5 * -80 + 0.5 * -60 = -70
        assert!((v + 70.0).abs() < f64::EPSILON);
    }

    #[test]
    fn reset_clears_state() {
        let mut ema = EmaFilter::new(0.2);
        ema.update(-50.0);
        ema.reset();
        assert!(ema.current().is_none());
    }
}
