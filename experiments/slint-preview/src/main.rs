mod session;

use obdium::scalar::{Scalar, Unit};
use obdium::OBD;
use session::PreviewSession;
use slint::ComponentHandle;
use std::cell::RefCell;

slint::include_modules!();

fn core_status(core: &OBD) -> &'static str {
    if core.is_connected() {
        "Core: unexpected connected state"
    } else {
        "Core: initialized, disconnected"
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = PreviewWindow::new()?;
    let core = OBD::new();
    ui.set_core_status(core_status(&core).into());
    let state = RefCell::new(PreviewSession::default());
    ui.set_rpm_text(state.borrow().rpm_text().into());

    let weak = ui.as_weak();
    ui.on_toggle_demo(move || {
        if let Some(ui) = weak.upgrade() {
            let mut state = state.borrow_mut();
            if state.is_active() {
                state.stop();
            } else if state.start() {
                if let Some(ticket) = state.request() {
                    let _ = state.complete(ticket, Scalar::new(1726.0, Unit::RPM, None));
                }
            }
            ui.set_demo_active(state.is_active());
            ui.set_rpm_text(state.rpm_text().into());
        }
    });

    ui.run()
}

#[cfg(test)]
mod core_boundary_tests {
    use super::*;
    use crate::session::format_synthetic_rpm;

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
