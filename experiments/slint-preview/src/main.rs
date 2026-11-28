mod session;
mod worker;

use obdium::OBD;
use session::{Permit, PreviewSession};
use slint::{ComponentHandle, Timer, TimerMode};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc::{TryRecvError, TrySendError};
use std::time::{Duration, Instant};
use worker::{Completion, DemoWorker};

slint::include_modules!();

fn core_status(core: &OBD) -> &'static str {
    if core.is_connected() {
        "Core: unexpected connected state"
    } else {
        "Core: initialized, disconnected"
    }
}

fn render_session(ui: &PreviewWindow, state: &PreviewSession) {
    ui.set_demo_active(state.is_active());
    ui.set_sample_available(state.can_request());
    ui.set_rpm_text(state.rpm_text().into());
}

// Refresh even if a rejected reply changed the visible request state.
fn accept_completion(state: &mut PreviewSession, reply: Completion, now: Instant) -> bool {
    let previous = state.rpm_text();
    let expired = state.expire(now);
    let accepted = state.complete(reply.ticket, reply.sample, now);
    expired || accepted || state.rpm_text() != previous
}

#[derive(Debug, PartialEq, Eq)]
enum WorkerEvent {
    Unchanged,
    Changed,
    Disconnected,
}

fn handle_worker_event(
    state: &mut PreviewSession,
    result: Result<Completion, TryRecvError>,
    now: Instant,
) -> WorkerEvent {
    match result {
        Ok(reply) => {
            if accept_completion(state, reply, now) {
                WorkerEvent::Changed
            } else {
                WorkerEvent::Unchanged
            }
        }
        Err(TryRecvError::Empty) => WorkerEvent::Unchanged,
        Err(TryRecvError::Disconnected) => {
            state.stop();
            WorkerEvent::Disconnected
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Dispatch {
    Submitted,
    NotReady,
    Busy,
    Disconnected,
    RequestFailed,
}

fn dispatch_sample(
    state: &mut PreviewSession,
    now: Instant,
    submit: impl FnOnce(Permit) -> Result<(), TrySendError<Permit>>,
) -> Dispatch {
    if !state.can_request() {
        return Dispatch::NotReady;
    }
    let Some(permit) = state.request(now).and_then(|ticket| state.permit(ticket)) else {
        state.stop();
        return Dispatch::RequestFailed;
    };
    let ticket = permit.ticket();
    match submit(permit) {
        Ok(()) => Dispatch::Submitted,
        Err(TrySendError::Full(_)) => {
            state.cancel(ticket);
            Dispatch::Busy
        }
        Err(TrySendError::Disconnected(_)) => {
            state.stop();
            Dispatch::Disconnected
        }
    }
}

fn dispatch_feedback(dispatch: &Dispatch) -> Option<&'static str> {
    match dispatch {
        Dispatch::Submitted => Some("Sample requested"),
        Dispatch::NotReady => None,
        Dispatch::Busy => Some("Demo worker busy; request again"),
        Dispatch::Disconnected => Some("Demo worker unavailable"),
        Dispatch::RequestFailed => Some("Could not start sample request"),
    }
}

fn stopped_demo_feedback(worker_available: bool) -> &'static str {
    if worker_available {
        "Demo stopped"
    } else {
        "Demo worker unavailable"
    }
}

fn request_demo_sample(
    ui: &PreviewWindow,
    state: &mut PreviewSession,
    worker_slot: &mut Option<DemoWorker>,
) {
    let Some(worker) = worker_slot.as_ref() else {
        state.stop();
        ui.set_demo_available(false);
        ui.set_demo_status("Demo worker unavailable".into());
        return;
    };
    let dispatch = dispatch_sample(state, Instant::now(), |permit| worker.submit(permit));
    if dispatch == Dispatch::Disconnected {
        worker_slot.take();
        ui.set_demo_available(false);
    }
    if let Some(feedback) = dispatch_feedback(&dispatch) {
        ui.set_demo_status(feedback.into());
    }
}

#[derive(Clone, Copy, Debug)]
enum SmokePhase {
    Startup,
    FirstSample,
    RepeatSample,
    Stopped(Instant),
    RestartSample,
    AwaitStale,
}

fn smoke_idle(ui: &PreviewWindow) -> bool {
    !ui.get_demo_active()
        && !ui.get_sample_available()
        && ui.get_rpm_text().as_str() == "Unavailable"
}

fn smoke_pending(ui: &PreviewWindow) -> bool {
    ui.get_demo_active()
        && !ui.get_sample_available()
        && ui.get_rpm_text().as_str() == "Waiting for sample"
}

fn smoke_sample(ui: &PreviewWindow) -> bool {
    ui.get_demo_active()
        && ui.get_sample_available()
        && ui.get_rpm_text().as_str() == "1726 rpm (synthetic)"
}

fn smoke_stale(ui: &PreviewWindow) -> bool {
    ui.get_demo_active()
        && ui.get_sample_available()
        && ui.get_rpm_text().as_str() == "Stale sample — request again"
}

// Exercise the actual generated callbacks and worker using the real event loop.
// This is synthetic-only and does not simulate physical keyboard/mouse input.
fn start_window_smoke(ui: &PreviewWindow, timer: &Timer, passed: Rc<Cell<bool>>) {
    let weak = ui.as_weak();
    let deadline = Instant::now() + Duration::from_secs(15);
    let phase = Cell::new(SmokePhase::Startup);
    timer.start(TimerMode::Repeated, Duration::from_millis(50), move || {
        let now = Instant::now();
        let current = phase.replace(SmokePhase::Startup);
        let mut next = None;
        if let Some(ui) = weak.upgrade().filter(|ui| {
            now < deadline
                && ui.get_demo_available()
                && ui.get_core_status().as_str() == "Core: initialized, disconnected"
        }) {
            match current {
                SmokePhase::Startup if !ui.window().is_visible() => {
                    next = Some(SmokePhase::Startup);
                }
                SmokePhase::Startup if smoke_idle(&ui) => {
                    ui.invoke_toggle_demo();
                    if smoke_pending(&ui) {
                        next = Some(SmokePhase::FirstSample);
                    }
                }
                SmokePhase::FirstSample if ui.window().is_visible() && smoke_pending(&ui) => {
                    next = Some(SmokePhase::FirstSample);
                }
                SmokePhase::FirstSample if ui.window().is_visible() && smoke_sample(&ui) => {
                    ui.invoke_request_sample();
                    if smoke_pending(&ui) {
                        next = Some(SmokePhase::RepeatSample);
                    }
                }
                SmokePhase::RepeatSample if ui.window().is_visible() && smoke_pending(&ui) => {
                    next = Some(SmokePhase::RepeatSample);
                }
                SmokePhase::RepeatSample if ui.window().is_visible() && smoke_sample(&ui) => {
                    ui.invoke_request_sample();
                    if smoke_pending(&ui) {
                        ui.invoke_toggle_demo();
                        if smoke_idle(&ui) {
                            next = Some(SmokePhase::Stopped(now));
                        }
                    }
                }
                SmokePhase::Stopped(stopped)
                    if ui.window().is_visible()
                        && smoke_idle(&ui)
                        && ui.get_demo_status().as_str() == "Demo stopped" =>
                {
                    if now
                        .checked_duration_since(stopped)
                        .is_some_and(|age| age >= Duration::from_millis(250))
                    {
                        ui.invoke_toggle_demo();
                        if smoke_pending(&ui) {
                            next = Some(SmokePhase::RestartSample);
                        }
                    } else {
                        next = Some(SmokePhase::Stopped(stopped));
                    }
                }
                SmokePhase::RestartSample if ui.window().is_visible() && smoke_pending(&ui) => {
                    next = Some(SmokePhase::RestartSample);
                }
                SmokePhase::RestartSample if ui.window().is_visible() && smoke_sample(&ui) => {
                    next = Some(SmokePhase::AwaitStale);
                }
                SmokePhase::AwaitStale if ui.window().is_visible() && smoke_sample(&ui) => {
                    next = Some(SmokePhase::AwaitStale);
                }
                SmokePhase::AwaitStale if ui.window().is_visible() && smoke_stale(&ui) => {
                    ui.invoke_toggle_demo();
                    passed.set(smoke_idle(&ui) && ui.get_demo_status().as_str() == "Demo stopped");
                }
                _ => {}
            }
        }
        if let Some(next) = next {
            phase.set(next);
            return;
        }
        if !passed.get() {
            eprintln!("Native window smoke failed in phase {current:?}");
        }
        if let Err(error) = slint::quit_event_loop() {
            passed.set(false);
            eprintln!("Window smoke shutdown failed: {error}");
        }
    });
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let smoke = std::env::args_os()
        .nth(1)
        .is_some_and(|arg| arg == "--smoke-test");
    let ui = PreviewWindow::new()?;
    let core = OBD::new();
    ui.set_core_status(core_status(&core).into());
    let state = Rc::new(RefCell::new(PreviewSession::default()));
    let (worker, thread) = DemoWorker::spawn()?;
    // Explicitly close both channels after the event loop, even if the UI
    // retains callback closures until component destruction.
    let worker = Rc::new(RefCell::new(Some(worker)));
    ui.set_demo_available(true);
    render_session(&ui, &state.borrow());

    let weak = ui.as_weak();
    let callback_state = Rc::clone(&state);
    let callback_worker = Rc::clone(&worker);
    ui.on_toggle_demo(move || {
        if let Some(ui) = weak.upgrade() {
            let mut state = callback_state.borrow_mut();
            let mut worker_slot = callback_worker.borrow_mut();
            if state.is_active() {
                state.stop();
                ui.set_demo_status(stopped_demo_feedback(worker_slot.is_some()).into());
            } else if worker_slot.is_some() {
                if state.start() {
                    request_demo_sample(&ui, &mut state, &mut worker_slot);
                } else {
                    ui.set_demo_status("Could not start demo session".into());
                }
            }
            render_session(&ui, &state);
        }
    });

    let weak = ui.as_weak();
    let callback_state = Rc::clone(&state);
    let callback_worker = Rc::clone(&worker);
    ui.on_request_sample(move || {
        if let Some(ui) = weak.upgrade() {
            let mut state = callback_state.borrow_mut();
            request_demo_sample(&ui, &mut state, &mut callback_worker.borrow_mut());
            render_session(&ui, &state);
        }
    });

    let timer = Timer::default();
    let weak = ui.as_weak();
    let timer_state = Rc::clone(&state);
    let timer_worker = Rc::clone(&worker);
    timer.start(TimerMode::Repeated, Duration::from_millis(50), move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        let mut state = timer_state.borrow_mut();
        let mut changed = state.expire(Instant::now());
        let mut worker_slot = timer_worker.borrow_mut();
        if let Some(worker) = worker_slot.as_ref() {
            let result = worker.receive();
            match handle_worker_event(&mut state, result, Instant::now()) {
                WorkerEvent::Unchanged => {}
                WorkerEvent::Changed => changed = true,
                WorkerEvent::Disconnected => {
                    worker_slot.take();
                    ui.set_demo_available(false);
                    ui.set_demo_status("Demo worker unavailable".into());
                    changed = true;
                }
            }
        }
        if changed {
            if worker_slot.is_some() {
                ui.set_demo_status("".into());
            }
            render_session(&ui, &state);
        }
    });

    let smoke_timer = Timer::default();
    let smoke_passed = Rc::new(Cell::new(false));
    if smoke {
        start_window_smoke(&ui, &smoke_timer, Rc::clone(&smoke_passed));
    }
    let result = ui.run();
    smoke_timer.stop();
    timer.stop();
    state.borrow_mut().stop();
    drop(worker.borrow_mut().take());
    thread
        .join()
        .map_err(|_| std::io::Error::other("synthetic demo worker panicked"))?;
    result?;
    if smoke {
        if !smoke_passed.get() {
            return Err(std::io::Error::other("Native window smoke check failed").into());
        }
        println!(
            "Native window smoke passed: startup, repeat, cancellation, restart, staleness and shutdown"
        );
    }
    Ok(())
}

#[cfg(test)]
mod core_boundary_tests {
    use super::*;
    use crate::session::Observation;
    use obdium::scalar::{Scalar, Unit};

    #[test]
    fn demo_feedback_does_not_claim_connection_or_sample_success() {
        assert_eq!(
            dispatch_feedback(&Dispatch::Submitted),
            Some("Sample requested")
        );
        assert_eq!(dispatch_feedback(&Dispatch::NotReady), None);
        assert_eq!(
            dispatch_feedback(&Dispatch::Busy),
            Some("Demo worker busy; request again")
        );
        assert_eq!(
            dispatch_feedback(&Dispatch::Disconnected),
            Some("Demo worker unavailable")
        );
        assert_eq!(
            dispatch_feedback(&Dispatch::RequestFailed),
            Some("Could not start sample request")
        );
        assert_eq!(stopped_demo_feedback(true), "Demo stopped");
        let unavailable = stopped_demo_feedback(false);
        assert_eq!(unavailable, "Demo worker unavailable");
    }

    #[test]
    fn linked_core_constructor_stays_disconnected() {
        let core = OBD::new();
        assert_eq!(core_status(&core), "Core: initialized, disconnected");
        assert_eq!(core.serial_port_name(), None);
    }

    #[test]
    fn sample_boundary_distinguishes_zero_and_invalid_values() {
        let zero = Scalar::new(0.0, Unit::RPM, None);
        assert_eq!(Observation::from_scalar(&zero).text(), "0 rpm (synthetic)");
        assert_eq!(
            Observation::from_scalar(&Scalar::no_data()),
            Observation::NoData
        );
        for sample in [
            Scalar::new(10.0, Unit::Volts, None),
            Scalar::new(f32::NAN, Unit::RPM, None),
            Scalar::new(f32::INFINITY, Unit::RPM, None),
            Scalar::new(-1.0, Unit::RPM, None),
        ] {
            assert_eq!(Observation::from_scalar(&sample), Observation::Invalid);
        }
    }

    #[test]
    fn rejected_clock_order_refreshes_pending_state_but_stale_reply_does_not() {
        let now = Instant::now();
        let mut state = PreviewSession::default();
        assert!(state.start());
        let old = state.request(now).expect("old ticket");
        assert!(state.cancel(old));
        let started = now + Duration::from_secs(1);
        let ticket = state.request(started).expect("current ticket");
        assert_eq!(state.rpm_text(), "Waiting for sample");
        assert!(!accept_completion(
            &mut state,
            Completion {
                ticket: old,
                sample: Observation::Rpm(100.0),
            },
            started
        ));
        assert_eq!(state.rpm_text(), "Waiting for sample");
        assert!(accept_completion(
            &mut state,
            Completion {
                ticket,
                sample: Observation::Rpm(100.0),
            },
            now
        ));
        assert_eq!(state.rpm_text(), "Invalid request clock");
    }

    #[test]
    fn worker_disconnection_is_visible_even_when_demo_is_inactive() {
        let now = Instant::now();
        let mut state = PreviewSession::default();
        assert_eq!(
            handle_worker_event(&mut state, Err(TryRecvError::Empty), now),
            WorkerEvent::Unchanged
        );
        assert_eq!(
            handle_worker_event(&mut state, Err(TryRecvError::Disconnected), now),
            WorkerEvent::Disconnected
        );
        assert!(!state.is_active());
        assert_eq!(state.rpm_text(), "Unavailable");
    }

    #[test]
    fn worker_disconnection_revokes_pending_request_and_rejects_late_sample() {
        let now = Instant::now();
        let mut state = PreviewSession::default();
        assert!(state.start());
        let ticket = state.request(now).expect("ticket");
        let permit = state.permit(ticket).expect("permit");
        assert_eq!(
            handle_worker_event(&mut state, Err(TryRecvError::Empty), now),
            WorkerEvent::Unchanged
        );
        assert!(permit.is_live(now));
        assert_eq!(
            handle_worker_event(&mut state, Err(TryRecvError::Disconnected), now),
            WorkerEvent::Disconnected
        );
        assert!(!permit.is_live(now));
        assert!(!state.is_active());
        assert_eq!(state.rpm_text(), "Unavailable");
        assert_eq!(
            handle_worker_event(
                &mut state,
                Ok(Completion {
                    ticket,
                    sample: Observation::Rpm(1726.0),
                }),
                now
            ),
            WorkerEvent::Unchanged
        );
    }

    #[test]
    fn manual_dispatch_is_single_flight_and_clears_previous_sample() {
        let now = Instant::now();
        let mut state = PreviewSession::default();
        assert_eq!(
            dispatch_sample(&mut state, now, |_| panic!("inactive session")),
            Dispatch::NotReady
        );
        assert!(state.start());
        let mut first = None;
        assert_eq!(
            dispatch_sample(&mut state, now, |permit| {
                first = Some(permit);
                Ok(())
            }),
            Dispatch::Submitted
        );
        let first = first.expect("first permit");
        assert!(!state.can_request());
        assert_eq!(
            dispatch_sample(&mut state, now, |_| panic!("duplicate request")),
            Dispatch::NotReady
        );
        assert!(state.complete(first.ticket(), Observation::Rpm(1726.0), now));
        assert!(state.can_request());
        assert_eq!(state.rpm_text(), "1726 rpm (synthetic)");
        assert_eq!(
            dispatch_sample(&mut state, now, |permit| {
                assert_ne!(permit.ticket(), first.ticket());
                Ok(())
            }),
            Dispatch::Submitted
        );
        assert_eq!(state.rpm_text(), "Waiting for sample");
        assert!(!state.complete(first.ticket(), Observation::Rpm(0.0), now));
    }

    #[test]
    fn full_queue_cancels_only_current_request_and_allows_manual_retry() {
        let now = Instant::now();
        let mut state = PreviewSession::default();
        assert!(state.start());
        let mut rejected = None;
        assert_eq!(
            dispatch_sample(&mut state, now, |permit| {
                rejected = Some(permit.clone());
                Err(TrySendError::Full(permit))
            }),
            Dispatch::Busy
        );
        assert!(state.is_active());
        assert!(state.can_request());
        assert_eq!(state.rpm_text(), "Unavailable");
        assert!(!rejected.expect("rejected permit").is_live(now));
        assert_eq!(
            dispatch_sample(&mut state, now, |_| Ok(())),
            Dispatch::Submitted
        );
    }

    #[test]
    fn disconnected_dispatch_revokes_permit_and_stops_session() {
        let now = Instant::now();
        let mut state = PreviewSession::default();
        assert!(state.start());
        let mut rejected = None;
        assert_eq!(
            dispatch_sample(&mut state, now, |permit| {
                rejected = Some(permit.clone());
                Err(TrySendError::Disconnected(permit))
            }),
            Dispatch::Disconnected
        );
        assert!(!state.is_active());
        assert!(!state.can_request());
        assert!(!rejected.expect("rejected permit").is_live(now));
        assert_eq!(state.rpm_text(), "Unavailable");
    }

    #[test]
    fn late_completion_updates_timeout_without_publishing_sample() {
        let now = Instant::now();
        let mut state = PreviewSession::default();
        assert!(state.start());
        let ticket = state.request(now).expect("ticket");
        let reply = Completion {
            ticket,
            sample: Observation::Rpm(1726.0),
        };
        assert!(accept_completion(
            &mut state,
            reply,
            now + Duration::from_secs(2)
        ));
        assert_eq!(state.rpm_text(), "Timed out");
        let duplicate = Completion {
            ticket,
            sample: Observation::Rpm(0.0),
        };
        assert!(!accept_completion(
            &mut state,
            duplicate,
            now + Duration::from_secs(2)
        ));
        assert_eq!(state.rpm_text(), "Timed out");
    }
}
