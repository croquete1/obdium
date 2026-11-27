use obdium::scalar::{Scalar, Unit};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);
// Standard OBD-II Mode 01 PID 0C: unsigned 16-bit value divided by four.
const MAX_STANDARD_RPM: f32 = u16::MAX as f32 / 4.0;

/// A ticket belongs to one demo session and one request within that session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Ticket {
    generation: u64,
    sequence: u64,
}

/// A worker receives only a permit for an already-issued observation.
#[derive(Clone, Debug)]
pub(crate) struct Permit {
    ticket: Ticket,
    started: Instant,
    deadline: Instant,
    cancelled: Arc<AtomicBool>,
    claimed: Arc<AtomicBool>,
}

impl Permit {
    pub(crate) fn ticket(&self) -> Ticket {
        self.ticket
    }

    pub(crate) fn is_live(&self, now: Instant) -> bool {
        self.remaining(now).is_some()
    }

    /// Sources must refresh this budget before each bounded I/O operation.
    pub(crate) fn remaining(&self, now: Instant) -> Option<Duration> {
        if self.cancelled.load(Ordering::Acquire) || now < self.started || now >= self.deadline {
            return None;
        }
        self.deadline.checked_duration_since(now)
    }

    /// Clones share the same execution claim: at most one worker may start
    /// this observation. A concurrent cancellation is checked again.
    pub(crate) fn claim(&self, now: Instant) -> bool {
        self.is_live(now)
            && self
                .claimed
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            && self.is_live(now)
    }

    fn invalidate(&self) {
        self.cancelled.store(true, Ordering::Release);
    }
}

/// Typed synthetic observation, distinct from request lifecycle state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Observation {
    Rpm(f32),
    NoData,
    Invalid,
    SourceTimeout,
    SourceDisconnected,
    Unsupported,
    SourceFailure,
}

impl Observation {
    pub(crate) fn from_scalar(sample: &Scalar) -> Self {
        if sample.unit == Unit::NoData {
            Self::NoData
        } else if sample.unit != Unit::RPM || !(0.0..=MAX_STANDARD_RPM).contains(&sample.value) {
            Self::Invalid
        } else {
            Self::Rpm(sample.value)
        }
    }

    pub(crate) fn text(self) -> String {
        match self {
            Self::Rpm(value) => format!("{value} rpm (synthetic)"),
            Self::NoData => "No data".into(),
            Self::Invalid => "Invalid measurement".into(),
            Self::SourceTimeout => "Source read timed out".into(),
            Self::SourceDisconnected => "Source disconnected".into(),
            Self::Unsupported => "Unsupported observation".into(),
            Self::SourceFailure => "Source read failed".into(),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct PreviewSession {
    generation: u64,
    sequence: u64,
    active: bool,
    pending: Option<Permit>,
    sample: Option<Observation>,
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
        if let Some(pending) = self.pending.as_ref() {
            self.cancel(pending.ticket);
        }
        self.sample = None;
        self.timed_out = false;
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active
    }

    pub(crate) fn can_request(&self) -> bool {
        self.active && self.pending.is_none()
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
        self.pending = Some(Permit {
            ticket,
            started: now,
            deadline,
            cancelled: Arc::new(AtomicBool::new(false)),
            claimed: Arc::new(AtomicBool::new(false)),
        });
        Some(ticket)
    }

    pub(crate) fn permit(&self, ticket: Ticket) -> Option<Permit> {
        self.pending
            .as_ref()
            .filter(|permit| permit.ticket == ticket)
            .cloned()
    }

    /// Accept only the currently outstanding request, exactly once.
    pub(crate) fn complete(&mut self, ticket: Ticket, sample: Observation, now: Instant) -> bool {
        let Some(pending) = self.pending.as_ref() else {
            return false;
        };
        if !self.active || pending.ticket != ticket {
            return false;
        }
        let started = pending.started;
        if self.expire(now) {
            return false;
        }
        if now < started {
            self.cancel(ticket);
            return false;
        }
        self.invalidate_pending();
        self.sample = Some(sample);
        true
    }

    /// The deadline is exclusive: completion at or after it is too late.
    pub(crate) fn expire(&mut self, now: Instant) -> bool {
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| now >= pending.deadline)
        {
            self.invalidate_pending();
            self.sample = None;
            self.timed_out = true;
            return true;
        }
        false
    }

    /// Explicit failure or cancellation for the matching request.
    pub(crate) fn cancel(&mut self, ticket: Ticket) -> bool {
        if !self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.ticket == ticket)
        {
            return false;
        }
        self.invalidate_pending();
        self.sample = None;
        self.timed_out = false;
        true
    }

    fn invalidate_pending(&mut self) {
        if let Some(pending) = self.pending.take() {
            pending.invalidate();
        }
    }

    pub(crate) fn rpm_text(&self) -> String {
        if self.timed_out {
            return "Timed out".into();
        }
        if self.pending.is_some() {
            return "Waiting for sample".into();
        }
        self.sample
            .map_or_else(|| "Unavailable".into(), Observation::text)
    }
}

impl Drop for PreviewSession {
    fn drop(&mut self) {
        self.invalidate_pending();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rpm(value: f32) -> Observation {
        Observation::from_scalar(&Scalar::new(value, Unit::RPM, None))
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
        assert_eq!(session.rpm_text(), "Waiting for sample");
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
        assert_eq!(session.rpm_text(), "Waiting for sample");
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
        assert_eq!(session.rpm_text(), "Waiting for sample");
        assert!(session.cancel(first));
        assert_eq!(session.rpm_text(), "Unavailable");
        assert!(!session.cancel(first));
        let second = session.request(now).expect("second");
        assert!(!session.complete(first, rpm(5000.0), now));
        assert!(session.complete(second, Observation::NoData, now));
        assert_eq!(session.rpm_text(), "No data");
    }

    #[test]
    fn counters_never_wrap_or_reuse_tickets() {
        let mut session = PreviewSession {
            generation: u64::MAX,
            sequence: 0,
            active: false,
            pending: None,
            sample: None,
            timed_out: false,
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
        assert_eq!(session.rpm_text(), "Waiting for sample");
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

    #[test]
    fn permits_are_revoked_by_stop_cancel_timeout_and_completion() {
        let now = Instant::now();
        let mut session = PreviewSession::default();
        assert!(session.start());
        let first = session.request(now).expect("first");
        let permit = session.permit(first).expect("permit");
        assert!(permit.is_live(now));
        session.stop();
        assert!(!permit.is_live(now));
        assert!(session.start());
        let second = session.request(now).expect("second");
        assert!(session.permit(first).is_none());
        let permit = session.permit(second).expect("permit");
        assert!(session.cancel(second));
        assert!(!permit.is_live(now));
        let third = session.request(now).expect("third");
        let permit = session.permit(third).expect("permit");
        assert!(session.expire(now + REQUEST_TIMEOUT));
        assert!(!permit.is_live(now));
        let fourth = session.request(now).expect("fourth");
        let permit = session.permit(fourth).expect("permit");
        assert!(session.complete(fourth, rpm(0.0), now));
        assert!(!permit.is_live(now));
    }

    #[test]
    fn cloned_permits_can_be_claimed_only_once() {
        let now = Instant::now();
        let mut session = PreviewSession::default();
        assert!(session.start());
        let ticket = session.request(now).expect("ticket");
        let permit = session.permit(ticket).expect("permit");
        let duplicate = permit.clone();
        assert!(permit.claim(now));
        assert!(!duplicate.claim(now));
        assert!(permit.is_live(now));
        drop(session);
        assert!(!duplicate.is_live(now));
    }

    #[test]
    fn typed_failure_cannot_replace_newer_observation() {
        let now = Instant::now();
        let mut session = PreviewSession::default();
        assert!(session.start());
        let old = session.request(now).expect("old");
        assert!(session.complete(old, Observation::SourceFailure, now));
        assert_eq!(session.rpm_text(), "Source read failed");
        let current = session.request(now).expect("current");
        assert_eq!(session.rpm_text(), "Waiting for sample");
        assert!(!session.complete(old, Observation::Invalid, now));
        assert!(session.complete(current, rpm(0.0), now));
        assert!(!session.complete(old, Observation::SourceFailure, now));
        assert_eq!(session.rpm_text(), "0 rpm (synthetic)");
    }

    #[test]
    fn remaining_budget_is_exclusive_and_revocable() {
        let now = Instant::now();
        let mut session = PreviewSession::default();
        assert!(session.start());
        let ticket = session.request(now).expect("ticket");
        let permit = session.permit(ticket).expect("permit");
        assert_eq!(permit.remaining(now), Some(REQUEST_TIMEOUT));
        assert_eq!(
            permit.remaining(now + Duration::from_secs(1)),
            Some(Duration::from_secs(1))
        );
        assert_eq!(permit.remaining(now + REQUEST_TIMEOUT), None);
        session.stop();
        assert_eq!(permit.remaining(now), None);
    }
}
