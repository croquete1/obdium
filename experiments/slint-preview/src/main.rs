mod session;
mod worker;

use obdium::OBD;
use session::PreviewSession;
use slint::{ComponentHandle, Timer, TimerMode};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{TryRecvError, TrySendError};
use std::time::{Duration, Instant};
use worker::DemoWorker;

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
    ui.set_rpm_text(state.rpm_text().into());
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = PreviewWindow::new()?;
    let core = OBD::new();
    ui.set_core_status(core_status(&core).into());
    let state = Rc::new(RefCell::new(PreviewSession::default()));
    let (worker, thread) = DemoWorker::spawn()?;
    // Explicitly close both channels after the event loop, even if the UI
    // retains callback closures until component destruction.
    let worker = Rc::new(RefCell::new(Some(worker)));
    render_session(&ui, &state.borrow());

    let weak = ui.as_weak();
    let callback_state = Rc::clone(&state);
    let callback_worker = Rc::clone(&worker);
    ui.on_toggle_demo(move || {
        if let Some(ui) = weak.upgrade() {
            let mut state = callback_state.borrow_mut();
            if state.is_active() {
                state.stop();
            } else if let Some(worker) = callback_worker.borrow().as_ref() {
                if state.start() {
                    if let Some(ticket) = state.request(Instant::now()) {
                        match worker.submit(ticket) {
                            Ok(()) => {
                                ui.set_core_status("Core: initialized, disconnected".into());
                            }
                            Err(TrySendError::Full(_)) => {
                                state.stop();
                                ui.set_core_status("Demo worker busy; try again".into());
                            }
                            Err(TrySendError::Disconnected(_)) => {
                                state.stop();
                                ui.set_core_status("Demo worker unavailable".into());
                            }
                        }
                    }
                }
            }
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
        let now = Instant::now();
        let mut changed = state.expire(now);
        if let Some(worker) = timer_worker.borrow().as_ref() {
            match worker.receive() {
                Ok(reply) => {
                    changed |= state.complete(reply.ticket, reply.sample, now);
                }
                Err(TryRecvError::Disconnected) if state.is_active() => {
                    state.stop();
                    ui.set_core_status("Demo worker unavailable".into());
                    changed = true;
                }
                Err(_) => {}
            }
        }
        if changed {
            render_session(&ui, &state);
        }
    });

    let result = ui.run();
    timer.stop();
    state.borrow_mut().stop();
    drop(worker.borrow_mut().take());
    thread
        .join()
        .map_err(|_| std::io::Error::other("synthetic demo worker panicked"))?;
    result?;
    Ok(())
}

#[cfg(test)]
mod core_boundary_tests {
    use super::*;
    use crate::session::format_synthetic_rpm;
    use obdium::scalar::{Scalar, Unit};

    #[test]
    fn linked_core_constructor_stays_disconnected() {
        let core = OBD::new();
        assert_eq!(core_status(&core), "Core: initialized, disconnected");
        assert_eq!(core.serial_port_name(), None);
    }

    #[test]
    fn sample_boundary_distinguishes_zero_and_invalid_values() {
        let zero = Scalar::new(0.0, Unit::RPM, None);
        assert_eq!(format_synthetic_rpm(&zero), "0 rpm (synthetic)");
        for sample in [
            Scalar::no_data(),
            Scalar::new(10.0, Unit::Volts, None),
            Scalar::new(f32::NAN, Unit::RPM, None),
            Scalar::new(f32::INFINITY, Unit::RPM, None),
            Scalar::new(-1.0, Unit::RPM, None),
        ] {
            assert_eq!(format_synthetic_rpm(&sample), "Unavailable");
        }
    }
}
