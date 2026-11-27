use obdium::scalar::{Scalar, Unit};
use std::time::{Duration, Instant};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);

/// A ticket belongs to one demo session and one request within that session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Ticket {
    generation: u64,
    sequence: u64,
}

#[derive(Clone, Copy, Debug)]
struct Pending {
    ticket: Ticket,
    started: Instant,
    deadline: Instant,
}

#[derive(Debug, Default)]
pub(crate) struct PreviewSession {
    generation: u64,
    sequence: u64,
    active: bool,
    pending: Option<Pending>,
    sample: Option<Scalar>,
    timed_out: bool,
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
        if let Some(pending) = self.pending {
            self.cancel(pending.ticket);
        }
        self.sample = None;
        self.timed_out = false;
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active
    }

    /// At most one request may be outstanding. Clear the old observation
    /// before asking for a new one.
    pub(crate) fn request(&mut self, now: Instant) -> Option<Ticket> {
        self.expire(now);
        if !self.active || self.pending.is_some() {
            return None;
        }
        let Some(sequence) = self.sequence.checked_add(1) else {
            self.stop();
            return None;
        };
        let Some(deadline) = now.checked_add(REQUEST_TIMEOUT) else {
            self.stop();
            return None;
        };
        self.sequence = sequence;
        let ticket = Ticket {
            generation: self.generation,
            sequence,
        };
        self.sample = None;
        self.timed_out = false;
        self.pending = Some(Pending {
            ticket,
            started: now,
            deadline,
        });
        Some(ticket)
    }

    /// Accept only the currently outstanding request, exactly once.
    pub(crate) fn complete(&mut self, ticket: Ticket, sample: Scalar, now: Instant) -> bool {
        let Some(pending) = self.pending else {
            return false;
        };
        if !self.active || pending.ticket != ticket {
            return false;
        }
        if self.expire(now) {
            return false;
        }
        if now < pending.started {
            self.cancel(ticket);
            return false;
        }
        self.pending = None;
        self.sample = Some(sample);
        true
    }

    /// The deadline is exclusive: completion at or after it is too late.
    pub(crate) fn expire(&mut self, now: Instant) -> bool {
        if self.pending.is_some_and(|pending| now >= pending.deadline) {
            self.pending = None;
            self.sample = None;
            self.timed_out = true;
            return true;
        }
        false
    }

    /// Explicit failure or cancellation for the matching request.
    pub(crate) fn cancel(&mut self, ticket: Ticket) -> bool {
        if !self.pending.is_some_and(|pending| pending.ticket == ticket) {
            return false;
        }
        self.pending = None;
        self.sample = None;
        self.timed_out = false;
        true
    }

    pub(crate) fn rpm_text(&self) -> String {
        if self.timed_out {
            return "Timed out".into();
        }
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
        let now = Instant::now();
        let mut session = PreviewSession::default();
        assert!(!session.is_active());
        assert!(session.request(now).is_none());
        assert_eq!(session.rpm_text(), "Unavailable");
    }

    #[test]
    fn demo_is_explicit_and_clears_sample_on_exit() {
        let now = Instant::now();
        let mut session = PreviewSession::default();
        assert!(session.start());
        assert!(session.is_active());
        let ticket = session.request(now).expect("request");
        assert!(session.complete(ticket, rpm(1726.0), now));
        assert_eq!(session.rpm_text(), "1726 rpm (synthetic)");
        session.stop();
        assert!(!session.is_active());
        assert_eq!(session.rpm_text(), "Unavailable");
        assert!(!session.complete(ticket, rpm(5000.0), now));
    }

    #[test]
    fn old_session_cannot_publish_into_new_session() {
        let now = Instant::now();
        let mut session = PreviewSession::default();
        assert!(session.start());
        let old = session.request(now).expect("old ticket");
        session.stop();
        assert!(session.start());
        let current = session.request(now).expect("current ticket");
        assert_ne!(old, current);
        assert!(!session.complete(old, rpm(5000.0), now));
        assert!(!session.cancel(old));
        assert_eq!(session.rpm_text(), "Unavailable");
        assert!(session.complete(current, rpm(0.0), now));
        assert_eq!(session.rpm_text(), "0 rpm (synthetic)");
    }

    #[test]
    fn outstanding_request_is_bounded_and_duplicates_are_rejected() {
        let now = Instant::now();
        let mut session = PreviewSession::default();
        assert!(session.start());
        let first = session.request(now).expect("first");
        assert!(session.request(now).is_none());
        assert!(session.complete(first, rpm(100.0), now));
        let second = session.request(now).expect("second");
        assert_eq!(session.rpm_text(), "Unavailable");
        assert!(!session.complete(first, rpm(5000.0), now));
        assert!(session.complete(second, rpm(200.0), now));
        assert!(!session.complete(second, rpm(5000.0), now));
        assert_eq!(session.rpm_text(), "200 rpm (synthetic)");
    }

    #[test]
    fn cancellation_invalidates_pending_sample() {
        let now = Instant::now();
        let mut session = PreviewSession::default();
        assert!(session.start());
        let first = session.request(now).expect("first");
        assert!(session.cancel(first));
        assert!(!session.cancel(first));
        let second = session.request(now).expect("second");
        assert!(!session.complete(first, rpm(5000.0), now));
        assert!(session.complete(second, Scalar::no_data(), now));
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
        let now = Instant::now();
        let mut session = PreviewSession::default();
        assert!(session.start());
        session.sequence = u64::MAX;
        assert!(session.request(now).is_none());
        assert!(!session.is_active());
        assert_eq!(session.rpm_text(), "Unavailable");
    }

    #[test]
    fn deadline_is_exclusive_and_does_not_turn_timeout_into_zero() {
        let now = Instant::now();
        let deadline = now + REQUEST_TIMEOUT;
        let mut session = PreviewSession::default();
        assert!(session.start());
        let first = session.request(now).expect("first");
        assert!(session.complete(first, rpm(0.0), deadline - Duration::from_nanos(1)));
        assert_eq!(session.rpm_text(), "0 rpm (synthetic)");
        let second = session.request(deadline).expect("second");
        let second_deadline = deadline + REQUEST_TIMEOUT;
        assert!(!session.complete(second, rpm(0.0), second_deadline));
        assert_eq!(session.rpm_text(), "Timed out");
        assert!(!session.complete(second, rpm(10.0), second_deadline));
        let third = session.request(second_deadline).expect("third");
        assert_eq!(session.rpm_text(), "Unavailable");
        assert!(!session.complete(
            third,
            rpm(10.0),
            second_deadline + REQUEST_TIMEOUT + Duration::from_nanos(1)
        ));
        assert_eq!(session.rpm_text(), "Timed out");
    }

    #[test]
    fn expiration_releases_slot_without_accepting_old_response() {
        let now = Instant::now();
        let deadline = now + REQUEST_TIMEOUT;
        let mut session = PreviewSession::default();
        assert!(session.start());
        let old = session.request(now).expect("old");
        assert!(!session.expire(deadline - Duration::from_nanos(1)));
        assert!(session.expire(deadline));
        assert!(!session.expire(deadline));
        assert_eq!(session.rpm_text(), "Timed out");
        let current = session.request(deadline).expect("current");
        assert!(!session.complete(old, rpm(100.0), deadline));
        assert!(session.complete(current, rpm(200.0), deadline));
        assert_eq!(session.rpm_text(), "200 rpm (synthetic)");
        session.stop();
        assert_eq!(session.rpm_text(), "Unavailable");
    }

    #[test]
    fn invalid_clock_order_and_stale_ticket_cannot_publish() {
        let now = Instant::now();
        let later = now + Duration::from_secs(1);
        let mut session = PreviewSession::default();
        assert!(session.start());
        let old = session.request(now).expect("old");
        assert!(session.cancel(old));
        let current = session.request(later).expect("current");
        assert!(!session.complete(old, rpm(100.0), later + REQUEST_TIMEOUT));
        assert!(!session.complete(current, rpm(200.0), now));
        assert_eq!(session.rpm_text(), "Unavailable");
        assert!(session.request(later).is_some());
    }
}
