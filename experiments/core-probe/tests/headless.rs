use obdium::{Response, OBD};

#[test]
fn core_starts_disconnected_without_opening_a_port() {
    let obd = OBD::new();
    assert!(!obd.is_connected());
    assert_eq!(obd.serial_port_name(), None);
    assert_eq!(obd.serial_port_baud_rate(), None);
}

#[test]
fn missing_response_remains_distinct_from_an_observed_value() {
    let response = Response::no_data();
    assert_eq!(response.full_response().as_deref(), Some("NO DATA"));
    assert_eq!(*response.get_payload_size(), 0);
}
