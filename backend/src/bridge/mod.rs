pub mod events;

use obdium::OBD;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tauri::{EventId, Listener, WebviewWindow};

pub static ACTIVE_OBD: Lazy<Mutex<Option<Arc<Mutex<OBD>>>>> = Lazy::new(|| Mutex::new(None));
// Serialize connection installation and teardown, not ordinary sensor reads.
pub static SESSION_TRANSITION: Mutex<()> = Mutex::new(());
pub static SESSION_LISTENERS: Mutex<Vec<EventId>> = Mutex::new(Vec::new());

fn take_matching<T>(active: &mut Option<Arc<T>>, expected: &Arc<T>) -> Option<Arc<T>> {
    if active.as_ref().is_some_and(|current| Arc::ptr_eq(current, expected)) {
        active.take()
    } else {
        None
    }
}

pub fn is_current_session(expected: &Arc<Mutex<OBD>>) -> bool {
    ACTIVE_OBD
        .lock()
        .unwrap()
        .as_ref()
        .is_some_and(|current| Arc::ptr_eq(current, expected))
}

pub fn take_current_session(expected: &Arc<Mutex<OBD>>) -> Option<Arc<Mutex<OBD>>> {
    take_matching(&mut ACTIVE_OBD.lock().unwrap(), expected)
}

pub(crate) static CUSTOM_PIDS_TRACKED: Lazy<Mutex<HashMap<String, CustomPid>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn unlisten_events(window: &Arc<WebviewWindow>) {
    let listeners = std::mem::take(&mut *SESSION_LISTENERS.lock().unwrap());
    for id in listeners {
        window.unlisten(id);
    }
}

#[cfg(test)]
mod session_tests {
    use super::*;

    #[test]
    fn stale_session_cannot_remove_replacement() {
        let old = Arc::new(1);
        let replacement = Arc::new(2);
        let mut active = Some(Arc::clone(&replacement));
        assert!(take_matching(&mut active, &old).is_none());
        assert!(Arc::ptr_eq(active.as_ref().unwrap(), &replacement));
    }

    #[test]
    fn current_session_can_be_removed_only_once() {
        let session = Arc::new(1);
        let mut active = Some(Arc::clone(&session));
        assert!(Arc::ptr_eq(
            &take_matching(&mut active, &session).unwrap(),
            &session
        ));
        assert!(take_matching(&mut active, &session).is_none());
    }

    #[test]
    fn equal_values_do_not_identify_the_same_session() {
        let old = Arc::new(1);
        let current = Arc::new(1);
        let mut active = Some(Arc::clone(&current));
        assert!(take_matching(&mut active, &old).is_none());
        assert!(take_matching(&mut active, &current).is_some());
    }
}

// Structs that are used as payloads between frontend and backend.

/// Very brief vehicle information
/// No detailed information- use VehicleInfoExtended
/// with the VIN struct for that instead.
#[derive(Serialize, Deserialize, Clone)]
struct VehicleInfo {
    vin: String,
    make: String,
    model: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct CustomPid {
    pub name: String,
    pub mode: String,
    pub pid: String,
    pub unit: String,
    pub command: String,
    pub equation: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct FrontendNotification {
    pub title: &'static str,
    pub description: &'static str,
}

/// All relevant information about a
/// vehicle.
///
/// Essential when the frontend
/// requests vehicle info about a VIN but
/// can't use the VIN struct and it's methods.
#[derive(Serialize, Deserialize, Clone, Default)]
struct VehicleInfoExtended {
    error_msg: String,
    vin: String,
    make: String,
    model: String,
    model_year: String,
    engine_model: String,
    cylinder_count: String,
    axel_count: String,
    traction_control: String,
    plant_country: String,
    plant_city: String,
    plant_state: String,
    semi_auto_headlamp_beam_switching: String,
    dynamic_brake_support: String,
    airbag_locations_knee: String,
    airbag_locations_side: String,
    drive_type: String,
    fuel_type: String,
    fuel_delivery_type: String,
    engine_manufacturer: String,
    anti_lock_braking_system: String,
    transmission_style: String,
    steering_location: String,
    keyless_ignition: String,
    top_speed: String,
    daytime_running_light: String,
    window_auto_reverse: String,
    airbag_locations_front: String,
    front_wheel_size: String,
    rear_wheel_size: String,
    automatic_crash_notification: String,
    trim: String,
    transmission_speeds: String,
    vehicle_base_price: String,
    number_of_rows: String,
    number_of_seats: String,
    brake_system: String,
    engine_displacement: String,
    gross_vehicle_weight_rating: String,
    airbag_locations_curtain: String,
    backup_camera: String,
    body_style: String,
    vehicle_manufacturer: String,
}

/// Tells the frontend the serial port
/// connection status.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct ConnectionStatus {
    connected: bool,
    message: String,
    serial_port: String,
    baud_rate: u32,
    protocol: u8,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConnectPaylod {
    serial_port: String,
    baud_rate: u32,
    protocol: u8,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Setting {
    t_id: String,
    checked: bool,

    // Optional data to send
    // Used for record_requests file pat
    data: Option<String>,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Dtc {
    category: String,
    description: String,
    name: String,
    permanant: bool,
    location: String,
}
