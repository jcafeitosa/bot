const MINUTE_MS: i64 = 60_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersistenceState {
    Off,
    Healthy,
    Degraded,
    Gap,
}

#[derive(Debug, Clone)]
pub struct PersistenceHealth {
    state: PersistenceState,
    initial_window_pending: bool,
    suspect_from: Option<i64>,
    last_confirmed: Option<i64>,
    latest_closed: Option<i64>,
    revision: u64,
}

impl PersistenceHealth {
    pub fn new(enabled: bool) -> Self {
        Self {
            state: if enabled {
                PersistenceState::Degraded
            } else {
                PersistenceState::Off
            },
            initial_window_pending: enabled,
            suspect_from: None,
            last_confirmed: None,
            latest_closed: None,
            revision: 0,
        }
    }

    pub fn state(&self) -> PersistenceState {
        self.state
    }
    #[cfg(test)]
    pub fn initial_window_pending(&self) -> bool {
        self.initial_window_pending
    }
    pub fn suspect_from(&self) -> Option<i64> {
        self.suspect_from
    }
    pub fn last_confirmed(&self) -> Option<i64> {
        self.last_confirmed
    }
    pub fn latest_closed(&self) -> Option<i64> {
        self.latest_closed
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn observe_closed(&mut self, timestamp: i64) {
        if self.state == PersistenceState::Off {
            return;
        }
        if let Some(last) = self.latest_closed {
            if timestamp > last.saturating_add(MINUTE_MS) {
                self.mark_suspect(last.saturating_add(MINUTE_MS));
            }
        }
        self.latest_closed = Some(
            self.latest_closed
                .map_or(timestamp, |last| last.max(timestamp)),
        );
    }

    pub fn observe_rest_window(&mut self, timestamps: &[i64]) {
        if self.state == PersistenceState::Off {
            return;
        }
        if let Some(first) = timestamps.first() {
            if self.suspect_from.is_some_and(|suspect| *first > suspect) {
                self.state = PersistenceState::Gap;
            }
        }
        for pair in timestamps.windows(2) {
            if pair[1] > pair[0].saturating_add(MINUTE_MS) {
                self.mark_suspect(pair[0].saturating_add(MINUTE_MS));
            }
        }
    }

    pub fn mark_suspect(&mut self, timestamp: i64) {
        if self.state == PersistenceState::Off {
            return;
        }
        self.suspect_from = Some(
            self.suspect_from
                .map_or(timestamp, |first| first.min(timestamp)),
        );
        self.revision = self.revision.wrapping_add(1);
        if self.state != PersistenceState::Gap {
            self.state = PersistenceState::Degraded;
        }
    }

    pub fn ws_confirmed(&mut self, timestamp: i64) {
        if self.state == PersistenceState::Off {
            return;
        }
        self.last_confirmed = Some(
            self.last_confirmed
                .map_or(timestamp, |last| last.max(timestamp)),
        );
    }

    pub fn ws_confirmed_at(&mut self, timestamp: i64, revision: u64, isolated_in_flight: bool) {
        self.ws_confirmed(timestamp);
        if isolated_in_flight
            && revision == self.revision
            && !self.initial_window_pending
            && self.suspect_from == Some(timestamp)
            && self.state != PersistenceState::Gap
        {
            self.suspect_from = None;
            self.state = PersistenceState::Healthy;
        }
    }

    #[cfg(test)]
    pub fn rest_confirmed(&mut self, timestamps: &[i64], latest_known_closed: i64) {
        self.rest_confirmed_at(timestamps, latest_known_closed, self.revision);
    }

    pub fn rest_confirmed_at(
        &mut self,
        timestamps: &[i64],
        latest_known_closed: i64,
        revision: u64,
    ) {
        if self.state == PersistenceState::Off || timestamps.is_empty() {
            return;
        }
        let first = timestamps[0];
        let last = *timestamps.last().expect("nonempty timestamps");
        self.last_confirmed = Some(self.last_confirmed.map_or(last, |prior| prior.max(last)));
        if self.state == PersistenceState::Gap || revision != self.revision {
            return;
        }
        if self.suspect_from.is_some_and(|suspect| first > suspect) {
            self.state = PersistenceState::Gap;
            return;
        }
        let contiguous = timestamps
            .windows(2)
            .all(|pair| pair[1] - pair[0] == MINUTE_MS);
        if contiguous
            && last >= latest_known_closed
            && self
                .suspect_from
                .is_none_or(|suspect| first <= suspect && last >= suspect)
        {
            self.initial_window_pending = false;
            self.suspect_from = None;
            self.state = PersistenceState::Healthy;
        }
    }

    pub fn label(&self) -> &'static str {
        match self.state {
            PersistenceState::Off => "OFF",
            PersistenceState::Healthy => "HEALTHY · sessão atual",
            PersistenceState::Degraded
                if self.initial_window_pending && self.suspect_from.is_none() =>
            {
                "DEGRADED · aguardando janela REST inicial"
            }
            PersistenceState::Degraded => "DEGRADED · recuperação REST pendente",
            PersistenceState::Gap => "GAP · reconciliação externa necessária",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINUTE: i64 = 60_000;

    #[test]
    fn initial_ws_success_does_not_establish_rest_baseline() {
        let mut health = PersistenceHealth::new(true);
        health.observe_closed(120_000);
        health.ws_confirmed(120_000);
        assert_eq!(health.state(), PersistenceState::Degraded);
        assert!(health.initial_window_pending());
        health.rest_confirmed(&[0, MINUTE, 120_000], 120_000);
        assert_eq!(health.state(), PersistenceState::Healthy);
    }

    #[test]
    fn ws_loss_needs_complete_confirmed_rest_recovery() {
        let mut health = PersistenceHealth::new(true);
        health.rest_confirmed(&[0, MINUTE], MINUTE);
        health.observe_closed(180_000);
        health.mark_suspect(120_000);
        health.rest_confirmed(&[MINUTE, 120_000], 180_000);
        assert_eq!(health.state(), PersistenceState::Degraded);
        health.rest_confirmed(&[120_000, 180_000], 180_000);
        assert_eq!(health.state(), PersistenceState::Healthy);
    }

    #[test]
    fn short_rest_window_enters_sticky_gap() {
        let mut health = PersistenceHealth::new(true);
        health.rest_confirmed(&[0, MINUTE], MINUTE);
        health.mark_suspect(120_000);
        health.rest_confirmed(&[180_000, 240_000], 240_000);
        assert_eq!(health.state(), PersistenceState::Gap);
        health.rest_confirmed(&[240_000, 300_000], 300_000);
        assert_eq!(health.state(), PersistenceState::Gap);
    }

    #[test]
    fn short_window_marks_gap_even_if_its_write_will_fail() {
        let mut health = PersistenceHealth::new(true);
        health.rest_confirmed(&[0, MINUTE], MINUTE);
        health.mark_suspect(120_000);
        health.observe_rest_window(&[180_000, 240_000]);
        assert_eq!(health.state(), PersistenceState::Gap);
    }

    #[test]
    fn rest_with_internal_hole_cannot_clear_suspect() {
        let mut health = PersistenceHealth::new(true);
        health.observe_rest_window(&[0, 120_000]);
        health.rest_confirmed(&[0, 120_000], 120_000);
        assert_eq!(health.state(), PersistenceState::Degraded);
        health.rest_confirmed(&[120_000, 180_000], 180_000);
        assert_eq!(health.state(), PersistenceState::Gap);
    }

    #[test]
    fn late_rest_completion_cannot_clear_new_pause_suspicion() {
        let mut health = PersistenceHealth::new(true);
        let attempt = health.revision();
        health.mark_suspect(120_000);
        health.rest_confirmed_at(&[0, MINUTE, 120_000], 120_000, attempt);
        assert_eq!(health.state(), PersistenceState::Degraded);
        assert_eq!(health.suspect_from(), Some(120_000));
    }

    #[test]
    fn successful_ws_does_not_clear_suspect_interval() {
        let mut health = PersistenceHealth::new(true);
        health.rest_confirmed(&[0, MINUTE], MINUTE);
        health.mark_suspect(120_000);
        let revision = health.revision();
        health.ws_confirmed_at(120_000, revision, false);
        assert_eq!(health.state(), PersistenceState::Degraded);
        assert_eq!(health.suspect_from(), Some(120_000));
    }

    #[test]
    fn confirmed_own_ws_write_preserves_health_after_rest_baseline() {
        let mut health = PersistenceHealth::new(true);
        health.rest_confirmed(&[0, MINUTE], MINUTE);
        assert_eq!(health.state(), PersistenceState::Healthy);
        health.mark_suspect(120_000);
        health.ws_confirmed_at(120_000, health.revision(), true);
        assert_eq!(health.state(), PersistenceState::Healthy);
    }

    #[test]
    fn rest_ending_before_suspect_minute_cannot_clear_pause() {
        let mut health = PersistenceHealth::new(true);
        health.rest_confirmed(&[0, MINUTE], MINUTE);
        health.mark_suspect(120_000);
        health.rest_confirmed(&[0, MINUTE], MINUTE);
        assert_eq!(health.state(), PersistenceState::Degraded);
        assert_eq!(health.suspect_from(), Some(120_000));
    }
}
