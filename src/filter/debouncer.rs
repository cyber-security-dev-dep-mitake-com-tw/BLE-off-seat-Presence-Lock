use std::time::{Duration, Instant};

/// Presence state after debouncing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenceState {
    Present,
    /// Below threshold; grace period counting down.
    Departing { remaining_secs: u64 },
    /// Grace period elapsed — caller should lock.
    Absent,
}

/// Grace-period debouncer: only reports Absent after RSSI stays below
/// threshold for `grace_period`.
#[derive(Debug, Clone)]
pub struct Debouncer {
    threshold: i16,
    grace_period: Duration,
    below_since: Option<Instant>,
    locked: bool,
}

impl Debouncer {
    pub fn new(threshold: i16, grace_period_secs: u64) -> Self {
        Self {
            threshold,
            grace_period: Duration::from_secs(grace_period_secs),
            below_since: None,
            locked: false,
        }
    }

    /// Feed a smoothed RSSI reading and return the current presence state.
    pub fn update(&mut self, smoothed_rssi: f64) -> PresenceState {
        let below = smoothed_rssi < self.threshold as f64;

        if !below {
            self.below_since = None;
            self.locked = false;
            return PresenceState::Present;
        }

        let since = *self.below_since.get_or_insert_with(Instant::now);
        let elapsed = since.elapsed();

        if elapsed >= self.grace_period {
            self.locked = true;
            PresenceState::Absent
        } else {
            let remaining = self.grace_period.saturating_sub(elapsed);
            PresenceState::Departing {
                remaining_secs: remaining.as_secs().saturating_add(
                    if remaining.subsec_nanos() > 0 { 1 } else { 0 },
                ),
            }
        }
    }

    /// Whether the last update concluded the device is absent.
    pub fn should_lock(&self) -> bool {
        self.locked
    }

    /// Reset grace timer and lock latch (e.g. after manual unlock / resume).
    pub fn reset(&mut self) {
        self.below_since = None;
        self.locked = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    pub fn stays_present_above_threshold() {
        let mut d = Debouncer::new(-80, 1);
        assert_eq!(d.update(-50.0), PresenceState::Present);
        assert!(!d.should_lock());
    }

    #[test]
    fn enters_departing_then_absent() {
        let mut d = Debouncer::new(-80, 1);
        let first = d.update(-90.0);
        assert!(matches!(first, PresenceState::Departing { .. }));
        assert!(!d.should_lock());

        thread::sleep(Duration::from_millis(1100));
        assert_eq!(d.update(-90.0), PresenceState::Absent);
        assert!(d.should_lock());
    }

    #[test]
    fn recovery_cancels_grace() {
        let mut d = Debouncer::new(-80, 5);
        let _ = d.update(-90.0);
        assert_eq!(d.update(-40.0), PresenceState::Present);
        assert!(!d.should_lock());
    }
}
