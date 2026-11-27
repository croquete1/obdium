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

    fn rpm_text(self) -> &'static str {
        match self {
            Self::Disconnected => "Unavailable",
            Self::SyntheticDemo => "1726 rpm (synthetic)",
        }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = PreviewWindow::new()?;
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
