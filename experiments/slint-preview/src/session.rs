use obdium::scalar::{Scalar, Unit};

/// A ticket belongs to one demo session and one request within that session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Ticket {
    generation: u64,
    sequence: u64,
}

#[derive(Debug, Default)]
pub(crate) struct PreviewSession {
    generation: u64,
    sequence: u64,
    active: bool,
    pending: Option<Ticket>,
    sample: Option<Scalar>,
}

impl PreviewSession {
    /// Starts a new synthetic session. Counter exhaustion fails closed.
    pub(crate) fn start(&mut self) -> bool {
        self.stop();
        let Some(generation) = self.generation.checked_add(1) else {
            return false;
        };
        self.generation = generation;
        self.sequence = 0;
        self.active = true;
        true
    }

    pub(crate) fn stop(&mut self) {
        self.active = false;
        if let Some(ticket) = self.pending {
            self.cancel(ticket);
        }
        self.sample = None;
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active
    }

    /// At most one request may be outstanding. Clear the old observation
    /// before asking for a new one.
    pub(crate) fn request(&mut self) -> Option<Ticket> {
        if !self.active || self.pending.is_some() {
            return None;
        }
        let Some(sequence) = self.sequence.checked_add(1) else {
            self.stop();
            return None;
        };
        self.sequence = sequence;
        let ticket = Ticket {
            generation: self.generation,
            sequence,
        };
        self.sample = None;
        self.pending = Some(ticket);
        Some(ticket)
    }

    /// Accept only the currently outstanding request, exactly once.
    pub(crate) fn complete(&mut self, ticket: Ticket, sample: Scalar) -> bool {
        if !self.active || self.pending != Some(ticket) {
            return false;
        }
        self.pending = None;
        self.sample = Some(sample);
        true
    }

    /// Explicit failure or cancellation for the matching request.
    pub(crate) fn cancel(&mut self, ticket: Ticket) -> bool {
        if self.pending != Some(ticket) {
            return false;
        }
        self.pending = None;
        self.sample = None;
        true
    }

    pub(crate) fn rpm_text(&self) -> String {
        self.sample
            .as_ref()
            .map_or_else(|| "Unavailable".into(), format_synthetic_rpm)
    }
}

pub(crate) fn format_synthetic_rpm(sample: &Scalar) -> String {
    if sample.unit != Unit::RPM || !sample.value.is_finite() || sample.value < 0.0 {
        return "Unavailable".into();
    }
    format!("{} rpm (synthetic)", sample.value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rpm(value: f32) -> Scalar {
        Scalar::new(value, Unit::RPM, None)
    }

    #[test]
    fn starts_disconnected_and_without_a_measurement() {
        let mut session = PreviewSession::default();
        assert!(!session.is_active());
        assert!(session.request().is_none());
        assert_eq!(session.rpm_text(), "Unavailable");
    }

    #[test]
    fn demo_is_explicit_and_clears_sample_on_exit() {
        let mut session = PreviewSession::default();
        assert!(session.start());
        assert!(session.is_active());
        let ticket = session.request().expect("request");
        assert!(session.complete(ticket, rpm(1726.0)));
        assert_eq!(session.rpm_text(), "1726 rpm (synthetic)");
        session.stop();
        assert!(!session.is_active());
        assert_eq!(session.rpm_text(), "Unavailable");
        assert!(!session.complete(ticket, rpm(5000.0)));
    }

    #[test]
    fn old_session_cannot_publish_into_new_session() {
        let mut session = PreviewSession::default();
        assert!(session.start());
        let old = session.request().expect("old ticket");
        session.stop();
        assert!(session.start());
        let current = session.request().expect("current ticket");
        assert_ne!(old, current);
        assert!(!session.complete(old, rpm(5000.0)));
        assert!(!session.cancel(old));
        assert_eq!(session.rpm_text(), "Unavailable");
        assert!(session.complete(current, rpm(0.0)));
        assert_eq!(session.rpm_text(), "0 rpm (synthetic)");
    }

    #[test]
    fn outstanding_request_is_bounded_and_duplicates_are_rejected() {
        let mut session = PreviewSession::default();
        assert!(session.start());
        let first = session.request().expect("first");
        assert!(session.request().is_none());
        assert!(session.complete(first, rpm(100.0)));
        let second = session.request().expect("second");
        assert_eq!(session.rpm_text(), "Unavailable");
        assert!(!session.complete(first, rpm(5000.0)));
        assert!(session.complete(second, rpm(200.0)));
        assert!(!session.complete(second, rpm(5000.0)));
        assert_eq!(session.rpm_text(), "200 rpm (synthetic)");
    }

    #[test]
    fn cancellation_invalidates_pending_sample() {
        let mut session = PreviewSession::default();
        assert!(session.start());
        let first = session.request().expect("first");
        assert!(session.cancel(first));
        assert!(!session.cancel(first));
        let second = session.request().expect("second");
        assert!(!session.complete(first, rpm(5000.0)));
        assert!(session.complete(second, Scalar::no_data()));
        assert_eq!(session.rpm_text(), "Unavailable");
    }

    #[test]
    fn counters_never_wrap_or_reuse_tickets() {
        let mut session = PreviewSession {
            generation: u64::MAX,
            ..Default::default()
        };
        assert!(!session.start());
        assert!(!session.is_active());
        let mut session = PreviewSession::default();
        assert!(session.start());
        session.sequence = u64::MAX;
        assert!(session.request().is_none());
        assert!(!session.is_active());
        assert_eq!(session.rpm_text(), "Unavailable");
    }
}
