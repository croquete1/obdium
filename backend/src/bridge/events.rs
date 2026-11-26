/// Events to bridge the frontend with
/// the backend.
///
/// Includes listening for requests from the frontend
/// to perform actions
///
use super::{
    ConnectPaylod, ConnectionStatus, Dtc, Setting, VehicleInfo, VehicleInfoExtended, ACTIVE_OBD,
};
use crate::bridge::{
    is_current_session, take_current_session, unlisten_events, CustomPid, CUSTOM_PIDS_TRACKED,
    SESSION_LISTENERS, SESSION_TRANSITION,
};
use crate::{connect_obd, track_data, OBD};
use obdium::dicts::PID_INFOS;
use obdium::scalar::UnitPreferences;
use obdium::vin::VIN;
use obdium::{Command, PAUSE_OBD_COUNT};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{async_runtime::spawn, WebviewWindow};
use tauri::{Emitter, EventId, Listener};
use tokio::time::sleep;

// "Listen" events
//
// Where the backend "listens" for events from the frontend
// All listen events are prefixed with 'listen'

pub fn listen_send_pids(window: &Arc<WebviewWindow>, obd: &Arc<Mutex<OBD>>) -> EventId {
    PAUSE_OBD_COUNT.fetch_add(1, Ordering::Relaxed);
    std::thread::sleep(Duration::from_millis(50));

    let window_arc = Arc::new(window.clone());
    let pids = {
        let mut obd = obd.lock().unwrap();
        obd.get_service_supported_pids("01")
    };

    println!("Supported pids list: \n{:?}", pids);

    let supported_pids: Vec<&String> = { pids.values().flatten().collect() };

    let mut supported_pids_info = PID_INFOS.to_owned();
    supported_pids_info
        .iter_mut()
        .for_each(|pid| pid.supported = supported_pids.contains(&&pid.pid.to_string()));
    supported_pids_info.sort_by_key(|b| std::cmp::Reverse(b.supported));

    if supported_pids_info.is_empty() {
        supported_pids_info = PID_INFOS.to_vec();
    }

    PAUSE_OBD_COUNT.fetch_sub(1, Ordering::Relaxed);
    println!("Supported pids: {supported_pids_info:?}");

    let obd_arc = Arc::clone(obd);
    window.listen("get-pids", move |_| {
        if is_current_session(&obd_arc) {
            let _ = window_arc.emit("update-pids", &supported_pids_info);
        }
    })
}

pub fn listen_track_custom_pid(window: &Arc<WebviewWindow>) {
    window.listen("track-custom-pid", move |event| {
        let payload = event.payload();
        println!("Request to track custom pid: {payload}");

        let mut pids = CUSTOM_PIDS_TRACKED.lock().unwrap();
        let custom_pid: CustomPid = match serde_json::from_str(payload) {
            Ok(p) => p,
            Err(e) => {
                println!("Failed to parse pid payload: {e}");
                return;
            }
        };

        // println!("Tracking PID {:?}", &custom_pid);
        pids.insert(custom_pid.name.clone(), custom_pid);
    });

    window.listen("remove-custom-pid", move |event| {
        let pid_name = event.payload();
        println!("Request to remove custom pid: {pid_name}");

        let mut pids = CUSTOM_PIDS_TRACKED.lock().unwrap();
        pids.remove(pid_name);
    });
}

pub fn listen_send_readiness_test(window: &Arc<WebviewWindow>, obd: &Arc<Mutex<OBD>>) -> EventId {
    let window_arc = Arc::new(window.clone());
    let obd_arc = Arc::clone(obd);
    let window_clone = Arc::clone(&window_arc);
    window_clone.listen("get-readiness-tests", move |_| {
        if !is_current_session(&obd_arc) {
            return;
        }
        PAUSE_OBD_COUNT.fetch_add(1, Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(50));

        let mut obd = obd_arc.lock().unwrap();
        if !is_current_session(&obd_arc) || !obd.is_connected() {
            PAUSE_OBD_COUNT.fetch_sub(1, Ordering::Relaxed);
            return;
        }
        let tests = obd.get_readiness_status();

        PAUSE_OBD_COUNT.fetch_sub(1, Ordering::Relaxed);

        if is_current_session(&obd_arc) {
            let _ = window_arc.emit("update-readiness-tests", tests);
        }
    })
}

pub fn listen_send_ports(window: &Arc<WebviewWindow>) {
    let window_arc = Arc::new(window.clone());
    let window_clone = Arc::clone(&window_arc);
    window_clone.listen("get-serial-ports", move |_| {
        println!("sending serial ports!");
        let _ = window_arc.emit("update-serial-ports", OBD::get_open_serial_ports());
    });
}

// TODO: This is an eye sore.
pub fn listen_decode_vin(window: &WebviewWindow) {
    let window_arc = Arc::new(window.clone());
    let window_clone = Arc::clone(&window_arc);
    window_clone.listen("decode-vin", move |event| {
        let vin = match VIN::new(event.payload().replace("\"", "")) {
            Ok(vin) => vin,
            Err(err) => {
                // emit error
                let v_info = VehicleInfoExtended {
                    error_msg: format!("{}", err),
                    ..Default::default()
                };

                let _ = window_arc.emit("decode-vin-result", v_info);
                return;
            }
        };

        let error_message = {
            if let Err(error) = vin.checksum() {
                format!("{}", error)
            } else {
                "Decoded successfully.".to_string()
            }
        };

        let v_info = VehicleInfoExtended {
            error_msg: error_message,
            vin: vin.get_vin().to_string(),
            make: vin.get_vehicle_make().unwrap_or("N/A".into()),
            model: vin.get_vehicle_model().unwrap_or("N/A".into()),
            model_year: vin.get_model_year().unwrap_or(-1).to_string(),
            engine_model: vin.get_engine_model().unwrap_or("N/A".into()),
            cylinder_count: vin.get_cylinder_count().unwrap_or(-1).to_string(),
            axel_count: vin.get_axle_count().unwrap_or(-1).to_string(),
            traction_control: vin.traction_control().unwrap_or("N/A".into()),
            plant_country: vin.get_plant_country().unwrap_or("N/A".into()),
            plant_city: vin.get_plant_city().unwrap_or("N/A".into()),
            plant_state: vin.get_plant_state().unwrap_or("N/A".into()),
            semi_auto_headlamp_beam_switching: vin
                .semiauto_headlamp_beam_switching()
                .unwrap_or("N/A".into()),
            dynamic_brake_support: vin.dynamic_brake_support().unwrap_or("N/A".into()),
            airbag_locations_knee: vin.airbag_locations_knee().unwrap_or("N/A".into()),
            airbag_locations_side: vin.airbag_locations_side().unwrap_or("N/A".into()),
            body_style: vin.get_body_class().unwrap_or("N/A".into()),
            fuel_type: vin.get_fuel_type().unwrap_or("N/A".into()),
            fuel_delivery_type: vin.get_fuel_delivery_type().unwrap_or("N/A".into()),
            engine_manufacturer: vin.get_engine_manufacturer().unwrap_or("N/A".into()),
            anti_lock_braking_system: vin.abs_availablility().unwrap_or("N/A".into()),
            transmission_style: vin.get_transmission_style().unwrap_or("N/A".into()),
            steering_location: vin.get_steering_location().unwrap_or("N/A".into()),
            keyless_ignition: vin.keyless_ignition_availability().unwrap_or("N/A".into()),
            top_speed: vin.get_vehicle_top_speed().unwrap_or(-1).to_string(),
            daytime_running_light: vin.daytime_running_light().unwrap_or("N/A".into()),
            window_auto_reverse: vin.windows_auto_reverse().unwrap_or("N/A".into()),
            airbag_locations_front: vin.airbag_locations_front().unwrap_or("N/A".into()),
            front_wheel_size: vin.get_front_wheel_size().unwrap_or(-1).to_string(),
            rear_wheel_size: vin.get_rear_wheel_size().unwrap_or(-1).to_string(),
            automatic_crash_notification: vin
                .automatic_crash_notification()
                .unwrap_or("N/A".into()),
            trim: vin.vehicle_trim().unwrap_or("N/A".into()),
            transmission_speeds: vin.get_transmission_speeds().unwrap_or(-1).to_string(),
            vehicle_base_price: vin.get_vehicle_base_price().unwrap_or(-1.0).to_string(),
            number_of_rows: vin.number_of_rows().unwrap_or(-1).to_string(),
            number_of_seats: vin.number_of_seats().unwrap_or(-1).to_string(),
            brake_system: vin.get_brake_system().unwrap_or("N/A".into()),
            engine_displacement: vin.get_engine_displacement().unwrap_or(-1.0).to_string(),
            gross_vehicle_weight_rating: vin.get_vehicle_weight_rating().unwrap_or("N/A".into()),
            airbag_locations_curtain: vin.airbag_locations_curtain().unwrap_or("N/A".into()),
            backup_camera: vin.backup_camera().unwrap_or("N/A".into()),
            drive_type: vin.get_drive_type().unwrap_or("N/A".into()),
            vehicle_manufacturer: vin.get_vehicle_manufacturer().unwrap_or("N/A".into()),
        };

        let _ = window_arc.emit("decode-vin-result", v_info);
    });
}

pub fn listen_connect_elm(window: &Arc<WebviewWindow>) {
    // The disconnect handler is application-scoped, not installed on every connection.
    listen_disconnect_elm(window);
    let window_arc = Arc::new(window.clone());
    let window_clone = Arc::clone(&window_arc);

    window_clone.listen("connect-elm", move |event| {
        let payload = event.payload();
        let connect_payload: ConnectPaylod = match serde_json::from_str(payload) {
            Ok(p) => p,
            Err(e) => {
                println!("Failed to parse connect payload: {e}");
                return;
            }
        };

        let _transition = SESSION_TRANSITION.lock().unwrap();
        if ACTIVE_OBD.lock().unwrap().is_some() {
            return;
        }

        let obd = connect_obd(
            &window_arc,
            connect_payload.serial_port,
            connect_payload.baud_rate,
            connect_payload.protocol,
        );

        if let Some(obd) = obd {
            let obd = Arc::new(Mutex::new(obd));

            {
                let mut active = ACTIVE_OBD.lock().unwrap();
                *active = Some(Arc::clone(&obd));
            }

            // Install all session listeners before announcing a ready connection.
            let listeners = vec![
                listen_set_unit_preferences(&window_arc, &obd),
                listen_send_connection_status(&window_arc, &obd),
                listen_change_obd_settings(&window_arc, &obd),
                listen_send_pids(&window_arc, &obd),
                listen_send_readiness_test(&window_arc, &obd),
                listen_send_dtcs(&window_arc, &obd),
                listen_run_user_command(&window_arc, &obd),
            ];
            *SESSION_LISTENERS.lock().unwrap() = listeners;

            {
                let connection = obd.lock().unwrap();
                let baud = connection.serial_port_baud_rate().unwrap_or_default();
                let port = connection.serial_port_name().unwrap_or_default();
                do_send_connection_status(
                    &window_arc,
                    &connection,
                    format!("Connected to port {port} on {baud} baud"),
                    connection.is_connected(),
                );
            }

            do_send_vehicle_details(&window_arc, &obd);
            track_data(&window_arc, &obd);

            let window_arc_clone = Arc::clone(&window_arc);
            let obd_clone = Arc::clone(&obd);
            spawn(async move {
                loop {
                    sleep(Duration::from_secs(1)).await;
                    if !is_current_session(&obd_clone) {
                        break;
                    }
                    if obd_clone.lock().unwrap().is_connected() {
                        continue;
                    }
                    let _transition = SESSION_TRANSITION.lock().unwrap();
                    if take_current_session(&obd_clone).is_some() {
                        unlisten_events(&window_arc_clone);
                        let connection = obd_clone.lock().unwrap();
                        do_send_connection_status(
                            &window_arc_clone,
                            &connection,
                            "Connection dropped".to_string(),
                            false,
                        );
                    }
                    break;
                }
            });
        }
    });
}

pub fn listen_disconnect_elm(window: &Arc<WebviewWindow>) {
    let window_arc_for_listen = Arc::clone(window);
    window_arc_for_listen.listen("disconnect-elm", {
        let window_arc_for_listen = Arc::clone(&window_arc_for_listen);
        move |_| {
            let _transition = SESSION_TRANSITION.lock().unwrap();
            PAUSE_OBD_COUNT.fetch_add(1, Ordering::Relaxed);
            std::thread::sleep(Duration::from_millis(50));

            let maybe_obd = {
                let mut active = ACTIVE_OBD.lock().unwrap();
                active.take()
            };

            if let Some(obd_arc) = maybe_obd {
                unlisten_events(&window_arc_for_listen);
                if let Ok(mut obd) = obd_arc.lock() {
                    obd.disconnect();
                    do_send_connection_status(
                        &window_arc_for_listen,
                        &obd,
                        "Connection dropped".to_string(),
                        false,
                    );
                }
            }

            PAUSE_OBD_COUNT.fetch_sub(1, Ordering::Relaxed);
        }
    });
}

pub fn listen_change_obd_settings(window: &Arc<WebviewWindow>, obd: &Arc<Mutex<OBD>>) -> EventId {
    let obd_arc = Arc::clone(obd);
    window.listen("settings-changed", move |event| {
        let payload = event.payload();
        let setting: Setting = match serde_json::from_str(payload) {
            Ok(p) => p,
            Err(e) => {
                println!("Failed to parse setting payload: {e}");
                return;
            }
        };

        let mut obd = obd_arc.lock().unwrap();
        if !is_current_session(&obd_arc) || !obd.is_connected() {
            return;
        }
        match setting.t_id.as_str() {
            "record-responses" => {
                if let Some(path) = setting.data {
                    println!("record response has a path: {path}");
                    obd.record_requests(setting.checked, path);
                } else {
                    obd.record_requests(setting.checked, "./data/requests.json".into());
                }
            }
            "replay-responses" => obd.replay_requests(setting.checked),
            // Freeze-frame selection is unavailable until snapshot identity is validated.
            _ => (),
        }
    })
}

pub fn listen_send_dtcs(window: &Arc<WebviewWindow>, obd: &Arc<Mutex<OBD>>) -> EventId {
    let obd_arc = Arc::clone(obd);
    let window_arc = Arc::clone(window);
    window.listen("get-dtcs", move |_| {
        let mut obd = obd_arc.lock().unwrap();
        if !is_current_session(&obd_arc) || !obd.is_connected() {
            return;
        }
        std::thread::sleep(Duration::from_secs(1));
        if !is_current_session(&obd_arc) {
            return;
        }

        let codes = [obd.get_trouble_codes(), obd.get_permanant_trouble_codes()].concat();
        println!("codes: {:?}", codes);

        // create serializable DTC struct from TroubleCode struct
        let serialized: Vec<Dtc> = codes
            .into_iter()
            .map(|dtc| Dtc {
                name: dtc.dtc,
                category: dtc.category.system_letter().to_string(),
                description: dtc.description,
                permanant: dtc.permanant,
                location: dtc.category.as_str().to_string(),
            })
            .collect();

        if is_current_session(&obd_arc) {
            let _ = window_arc.emit("update-dtcs", serialized);
        }
    })
}

// Intentionally no clear-dtcs event handler until Mode 04 acknowledgement,
// current-session authorization and reconciliation are implemented (F04.04).

pub fn listen_send_connection_status(
    window: &Arc<WebviewWindow>,
    obd: &Arc<Mutex<OBD>>,
) -> EventId {
    let obd_arc = Arc::clone(obd);
    let window_arc = Arc::clone(window);
    window.listen("get-connection-status", move |_| {
        let obd = obd_arc.lock().unwrap();
        if is_current_session(&obd_arc) {
            do_send_connection_status(&window_arc, &obd, "".into(), obd.is_connected());
        }
    })
}

pub fn listen_run_user_command(window: &Arc<WebviewWindow>, obd: &Arc<Mutex<OBD>>) -> EventId {
    let window_arc = Arc::clone(window);
    let obd_arc = Arc::clone(obd);
    window.listen("terminal-command", move |event| {
        PAUSE_OBD_COUNT.fetch_add(1, Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(50));

        let command = event.payload();

        {
            let mut obd = obd_arc.lock().unwrap();
            if !is_current_session(&obd_arc) || !obd.is_connected() {
                PAUSE_OBD_COUNT.fetch_sub(1, Ordering::Relaxed);
                return;
            }
            if let Err(err) = obd.send_command(&mut Command::new_arb(command)) {
                do_send_command_output(&window_arc, format!("Response: {}", err));
                PAUSE_OBD_COUNT.fetch_sub(1, Ordering::Relaxed);
                return;
            }

            match obd.get_pid_response() {
                Ok(response) => {
                    do_send_command_output(
                        &window_arc,
                        format!(
                            "Response: {}",
                            response
                                .raw_response()
                                .unwrap_or("n/a".to_string())
                                .replace("\n", " ")
                        ),
                    );
                }
                Err(err) => {
                    do_send_command_output(&window_arc, format!("Response: {}", err));
                }
            }
        }

        PAUSE_OBD_COUNT.fetch_sub(1, Ordering::Relaxed);
    })
}

pub fn listen_set_unit_preferences(window: &Arc<WebviewWindow>, obd: &Arc<Mutex<OBD>>) -> EventId {
    let obd_arc = Arc::clone(obd);
    window.listen("set-unit-preferences", move |event| {
        let payload = event.payload();
        println!("Setting unit preference to {}", payload);

        let mut obd = obd_arc.lock().unwrap();
        if !is_current_session(&obd_arc) || !obd.is_connected() {
            return;
        }
        let unit_preferences: UnitPreferences = match serde_json::from_str(payload) {
            Ok(unit_preferences) => unit_preferences,
            Err(err) => {
                println!("error serializing UnitPreferences from frontned: {err}");
                return;
            }
        };

        obd.set_unit_preferences(unit_preferences);
    })
}

// Calls to the frontend from the backend
// where the backend umprompty tells the
// frontend to do something.
//
// In this case, the frontend would be listening for these events.

pub fn do_send_command_output(window: &Arc<WebviewWindow>, msg: String) {
    let _ = window.emit("update-command-output", msg);
}

pub fn do_send_vehicle_details(window: &Arc<WebviewWindow>, obd: &Arc<Mutex<OBD>>) {
    let obd = Arc::clone(obd);
    let window = Arc::clone(window);
    spawn(async move {
        PAUSE_OBD_COUNT.fetch_add(1, Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(4000));
        let mut obd = obd.lock().unwrap();

        // send the vin and vehicle details to the frontend
        match obd.get_vin() {
            Some(vin) => {
                let make = match vin.get_vehicle_make() {
                    Ok(make) => make,
                    Err(err) => {
                        println!(
                            "failed to resolve vehicle make from vin: '{}'",
                            vin.get_vin()
                        );
                        println!("error: {err}");
                        "??".to_string()
                    }
                };

                let model = match vin.get_vehicle_model() {
                    Ok(model) => model,
                    Err(err) => {
                        println!(
                            "failed to resolve vehicle model from vin: '{}'",
                            vin.get_vin()
                        );
                        println!("error: {err}");
                        "??".to_string()
                    }
                };

                let v_info = VehicleInfo {
                    vin: vin.get_vin().to_string(),
                    make,
                    model,
                };

                window.emit("vehicle-details", v_info).unwrap();
            }
            None => {
                println!("error: getting vin. vin is none.");
            }
        };

        PAUSE_OBD_COUNT.fetch_sub(1, Ordering::Relaxed);
    });
}

pub fn do_send_connection_status(
    window: &WebviewWindow,
    obd: &OBD,
    message: String,
    connected: bool,
) {
    let port = obd.serial_port_name().unwrap_or_default();
    let baud = obd.serial_port_baud_rate().unwrap_or_default();
    let conn_status = ConnectionStatus {
        connected,
        message,
        serial_port: port,
        baud_rate: baud,
        protocol: obd.get_protocol_number(),
    };

    println!("connection status: {conn_status:?}");
    let _ = window.emit("connection-status", conn_status);
}
