use obdium::scalar::{Scalar, Unit};
use obdium::OBD;
use slint::ComponentHandle;
use std::cell::Cell;

slint::include_modules!();

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum PreviewState {
    #[default]
    Disconnected,
    SyntheticDemo,
}

impl PreviewState {
    fn toggle(self) -> Self {
        match self {
            Self::Disconnected => Self::SyntheticDemo,
            Self::SyntheticDemo => Self::Disconnected,
        }
    }

    fn demo_active(self) -> bool {
        self == Self::SyntheticDemo
    }

    fn sample(self) -> Scalar {
        match self {
            Self::Disconnected => Scalar::no_data(),
            Self::SyntheticDemo => Scalar::new(1726.0, Unit::RPM, None),
        }
    }

    fn rpm_text(self) -> String {
        format_synthetic_rpm(&self.sample())
    }
}

fn format_synthetic_rpm(sample: &Scalar) -> String {
    if sample.unit != Unit::RPM || !sample.value.is_finite() || sample.value < 0.0 {
        return "Unavailable".into();
    }
    format!("{} rpm (synthetic)", sample.value)
}

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
    let state = Cell::new(PreviewState::default());
    ui.set_rpm_text(state.get().rpm_text().into());

    let weak = ui.as_weak();
    ui.on_toggle_demo(move || {
        if let Some(ui) = weak.upgrade() {
            let next = state.get().toggle();
            state.set(next);
            ui.set_demo_active(next.demo_active());
            ui.set_rpm_text(next.rpm_text().into());
        }
    });

    ui.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_starts_disconnected_and_without_a_measurement() {
        let state = PreviewState::default();
        assert!(!state.demo_active());
        assert_eq!(state.rpm_text(), "Unavailable");
    }

    #[test]
    fn demo_is_explicit_and_clears_sample_on_exit() {
        let demo = PreviewState::default().toggle();
        assert!(demo.demo_active());
        assert_eq!(demo.rpm_text(), "1726 rpm (synthetic)");
        let stopped = demo.toggle();
        assert!(!stopped.demo_active());
        assert_eq!(stopped.rpm_text(), "Unavailable");
    }
}

#[cfg(test)]
mod core_boundary_tests {
    use super::*;

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
