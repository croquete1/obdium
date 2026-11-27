use serialport::SerialPort;
use sqlite::State;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::str::{self, FromStr};
use std::thread::sleep;
use std::time::{Duration, Instant};
use thiserror::Error;

use crate::cmd::{Command, CommandType};
use crate::protocol::{
    can_probe_identifier_width, confirm_can_probe, confirm_legacy_probe, parse_reported_protocol,
    validate_selection_ack, LegacyProbe, ObdProtocol, ProtocolSelection, ReportedProtocol,
};
use crate::response::Response;
use crate::scalar::{Scalar, Unit, UnitPreferences};
use crate::vin::VIN;
use crate::MODE22_PIDS_DB_PATH;

#[derive(Debug)]
pub enum BankNumber {
    Bank1,
    Bank2,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SensorNumber {
    Sensor1,
    Sensor2,
    Sensor3,
    Sensor4,
    Sensor5,
    Sensor6,
    Sensor7,
    Sensor8,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to establish connection with adapter")]
    ConnectionFailed,
    #[error("Attempted to perform a read/write action via serial connection, but no serial connection to adapter is active")]
    NoConnection,

    #[error("Failed to initialize connection via adapter sending command: {0}")]
    InitFailedSendingCommand(String),
    #[error("Could not initialize connection via adapter - Failed reading response")]
    InitFailedReadingResponse,
    #[error("No response containing 'OK' received from adapter while initializing connection")]
    InitFailedNoOKReceived,

    #[error("Invalid OBD protocol given while trying to connect. Expected number between 0-9, got: {0} ")]
    InvalidProtocol(u8),
    #[error("Received invalid or mangled response to command via serial connection")]
    InvalidResponse,
    #[error(
        "Received response explicitly containing 'NODATA' thus no meaningful data was received"
    )]
    NoData,
    #[error("No positive response received after sending OBD request to clear DTCs. Expected positive response '44' but got '{0}'")]
    DTCClearFailed(String),

    #[error(
        "Error occured during I/O operation via serial communication. Error details (if any): {0}"
    )]
    AdapterReadWriteError(String),
    #[error("Adapter reply exceeded the total receive deadline")]
    ReplyDeadlineExceeded,
}

// Limit a single adapter reply to avoid unbounded allocation if its prompt is lost.
// Large/continuous diagnostic streams require a separate framing design.
const MAX_ADAPTER_REPLY_BYTES: usize = 64 * 1024;
const ADAPTER_IDLE_TIMEOUT: Duration = Duration::from_secs(1);
const ADAPTER_REPLY_DEADLINE: Duration = Duration::from_secs(5);

fn map_serial_error(error: serialport::Error) -> Error {
    if error.kind() == serialport::ErrorKind::NoDevice {
        Error::NoConnection
    } else {
        Error::AdapterReadWriteError(error.to_string())
    }
}

/// Read a complete prompt-terminated reply without clearing or discarding input.
/// A partial reply is never accepted on EOF, timeout or I/O failure.
fn read_prompt_with_deadline(
    until: u8,
    mut elapsed: impl FnMut() -> Duration,
    mut read: impl FnMut(&mut [u8], Duration) -> Result<usize, Error>,
) -> Result<String, Error> {
    let mut response = String::new();
    let mut buffer = [0u8; 1];

    loop {
        let remaining = ADAPTER_REPLY_DEADLINE.saturating_sub(elapsed());
        if remaining.is_zero() {
            return Err(Error::ReplyDeadlineExceeded);
        }
        let result = read(&mut buffer, remaining.min(ADAPTER_IDLE_TIMEOUT));
        if elapsed() >= ADAPTER_REPLY_DEADLINE {
            return Err(Error::ReplyDeadlineExceeded);
        }
        match result {
            Ok(1) => {
                let byte = buffer[0];
                if byte == until {
                    return Ok(response);
                }
                if !byte.is_ascii() || response.len() >= MAX_ADAPTER_REPLY_BYTES {
                    return Err(Error::InvalidResponse);
                }
                response.push(char::from(byte));
            }
            Ok(_) => return Err(Error::InvalidResponse),
            Err(error) => return Err(error),
        }
    }
}

fn read_serial_prompt(port: &mut dyn SerialPort, until: u8) -> Result<String, Error> {
    let start = Instant::now();
    let mut timeout = port.timeout();
    read_prompt_with_deadline(
        until,
        || start.elapsed(),
        |buffer, remaining| {
            // Avoid reconfiguring the port for every byte while the full idle budget remains.
            if timeout != remaining {
                port.set_timeout(remaining).map_err(map_serial_error)?;
                timeout = remaining;
            }
            port.read(buffer)
                .map_err(|error| Error::AdapterReadWriteError(error.to_string()))
        },
    )
}

#[cfg(test)]
fn read_prompt<R: Read + ?Sized>(reader: &mut R, until: u8) -> Result<String, Error> {
    read_prompt_with_deadline(
        until,
        || Duration::ZERO,
        |buffer, _| {
            reader
                .read(buffer)
                .map_err(|error| Error::AdapterReadWriteError(error.to_string()))
        },
    )
}

pub enum Service {
    Mode01,
    Mode22,
}

// Fake serial port used for simulating.
// Specifically demo mode.
struct DummySerialPort;

impl SerialPort for DummySerialPort {
    fn name(&self) -> Option<String> {
        Some("DEMO MODE".to_string())
    }
    fn baud_rate(&self) -> serialport::Result<u32> {
        Ok(0)
    }
    fn data_bits(&self) -> serialport::Result<serialport::DataBits> {
        Ok(serialport::DataBits::Eight)
    }
    fn flow_control(&self) -> serialport::Result<serialport::FlowControl> {
        Ok(serialport::FlowControl::None)
    }
    fn parity(&self) -> serialport::Result<serialport::Parity> {
        Ok(serialport::Parity::None)
    }
    fn stop_bits(&self) -> serialport::Result<serialport::StopBits> {
        Ok(serialport::StopBits::One)
    }
    fn timeout(&self) -> Duration {
        Duration::from_secs(0)
    }
    fn set_timeout(&mut self, _timeout: Duration) -> serialport::Result<()> {
        Ok(())
    }
    fn write_request_to_send(&mut self, _level: bool) -> serialport::Result<()> {
        Ok(())
    }
    fn write_data_terminal_ready(&mut self, _level: bool) -> serialport::Result<()> {
        Ok(())
    }
    fn read_clear_to_send(&mut self) -> serialport::Result<bool> {
        Ok(true)
    }
    fn read_data_set_ready(&mut self) -> serialport::Result<bool> {
        Ok(true)
    }
    fn read_ring_indicator(&mut self) -> serialport::Result<bool> {
        Ok(false)
    }
    fn read_carrier_detect(&mut self) -> serialport::Result<bool> {
        Ok(false)
    }
    fn bytes_to_read(&self) -> serialport::Result<u32> {
        Ok(0)
    }
    fn bytes_to_write(&self) -> serialport::Result<u32> {
        Ok(0)
    }
    fn clear(&self, _buffer: serialport::ClearBuffer) -> serialport::Result<()> {
        Ok(())
    }
    fn try_clone(&self) -> serialport::Result<Box<dyn SerialPort>> {
        Ok(Box::new(DummySerialPort))
    }
    fn set_baud_rate(&mut self, _baud_rate: u32) -> serialport::Result<()> {
        Ok(())
    }
    fn set_data_bits(&mut self, _data_bits: serialport::DataBits) -> serialport::Result<()> {
        Ok(())
    }
    fn set_flow_control(
        &mut self,
        _flow_control: serialport::FlowControl,
    ) -> serialport::Result<()> {
        Ok(())
    }
    fn set_parity(&mut self, _parity: serialport::Parity) -> serialport::Result<()> {
        Ok(())
    }
    fn set_stop_bits(&mut self, _stop_bits: serialport::StopBits) -> serialport::Result<()> {
        Ok(())
    }
    fn set_break(&self) -> serialport::Result<()> {
        Ok(())
    }
    fn clear_break(&self) -> serialport::Result<()> {
        Ok(())
    }
}

impl Read for DummySerialPort {
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(0)
    }
}

impl Write for DummySerialPort {
    fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
        Ok(0)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[derive(Default)]
pub struct OBD {
    connection: Option<Box<dyn SerialPort>>,
    elm_version: Option<String>,
    freeze_frame_query: bool,
    protocol: u8,

    pub(crate) requests_path: String,
    pub(crate) record_requests: bool,
    pub(crate) replay_requests: bool,

    pub(crate) unit_preferences: UnitPreferences,
}

impl OBD {
    pub fn new() -> Self {
        Self {
            requests_path: "./data/requests.json".to_string(),
            ..Default::default()
        }
    }

    pub fn connect(&mut self, port: &str, baud_rate: u32, protocol: u8) -> Result<(), Error> {
        if self.connection.is_some() {
            return Ok(());
        }

        self.protocol = 0;
        self.elm_version = None;
        self.freeze_frame_query = false;

        if port == "DEMO MODE" {
            // No connection required
            self.replay_requests = true;
            self.connection = Some(Box::new(DummySerialPort));

            return Ok(());
        }

        self.replay_requests = false;
        self.record_requests = false;

        let selection =
            ProtocolSelection::try_from(protocol).map_err(|_| Error::InvalidProtocol(protocol))?;

        self.connection = serialport::new(port, baud_rate)
            .timeout(Duration::from_secs(1))
            .open()
            .ok();

        if !self.is_connected() {
            return Err(Error::ConnectionFailed);
        }

        self.initialize_protocol(selection, protocol)
    }

    fn initialize_protocol(
        &mut self,
        selection: ProtocolSelection,
        protocol: u8,
    ) -> Result<(), Error> {
        if let Err(error) = self.init().and_then(|()| self.select_protocol(selection)) {
            self.disconnect();
            return Err(error);
        }

        // This is the selected configuration, not the negotiated ECU protocol.
        self.protocol = protocol;
        Ok(())
    }

    fn select_protocol(&mut self, selection: ProtocolSelection) -> Result<(), Error> {
        self.send_command(&mut Command::new_at(selection.command()))?;
        let response = self.get_at_response()?;
        validate_selection_ack(response.raw_response.as_deref().unwrap_or_default())
            .map_err(|_| Error::InitFailedNoOKReceived)
    }

    pub fn disconnect(&mut self) {
        self.connection = None;
        self.protocol = 0;
        self.elm_version = None;
        self.freeze_frame_query = false;
    }

    pub fn is_connected(&self) -> bool {
        self.connection.is_some()
    }

    pub fn serial_port_name(&self) -> Option<String> {
        match &self.connection {
            Some(connection) => connection.name(),
            None => None,
        }
    }

    pub fn serial_port_baud_rate(&self) -> Option<u32> {
        match &self.connection {
            Some(connection) => connection.baud_rate().ok(),
            None => None,
        }
    }

    pub fn get_open_serial_ports() -> Vec<(String, u32)> {
        let ports = match serialport::available_ports() {
            Ok(p) => p,
            Err(err) => {
                println!("error getting available serial ports: {err}");
                return Vec::new();
            }
        };

        const BAUD_RATES: [u32; 6] = [38400, 9600, 115200, 57600, 230400, 4800];
        let handles: Vec<_> = ports
            .into_iter()
            .map(|port| {
                std::thread::spawn(move || {
                    let port_name = port.port_name;
                    for baud_rate in BAUD_RATES.iter() {
                        let Ok(mut con) = serialport::new(&port_name, *baud_rate)
                            .timeout(Duration::from_millis(500))
                            .open()
                        else {
                            continue;
                        };

                        let _ = con.clear(serialport::ClearBuffer::All);

                        if con.write_all(b"ATZ\r").is_err() {
                            continue;
                        }

                        std::thread::sleep(Duration::from_millis(500));
                        let _ = con.clear(serialport::ClearBuffer::All);

                        if con.write_all(b"ATI\r").is_err() {
                            continue;
                        }

                        let mut buffer: [u8; 256] = [0u8; 256];
                        let mut total_read = 0;
                        loop {
                            match con.read(&mut buffer[total_read..]) {
                                Ok(n) if n > 0 => total_read += n,
                                Ok(_) => break,
                                Err(e)
                                    if e.kind() == std::io::ErrorKind::TimedOut
                                        || e.kind() == std::io::ErrorKind::WouldBlock =>
                                {
                                    break
                                }
                                Err(_) => break,
                            }
                        }

                        let response = String::from_utf8_lossy(&buffer[..total_read]);
                        let cleaned = response.trim_matches(|c: char| !c.is_ascii_graphic());

                        println!(
                            "- response: `{}` for port: {}",
                            cleaned.escape_debug(),
                            port_name
                        );

                        if cleaned.ends_with('>') || cleaned.contains("ELM") {
                            return Some((port_name, *baud_rate));
                        }
                    }
                    None
                })
            })
            .collect();

        let open_serial_ports: Vec<(String, u32)> = handles
            .into_iter()
            .filter_map(|h| h.join().ok().flatten())
            .collect();

        println!("Ports: {:?}", open_serial_ports);
        open_serial_ports
    }

    pub fn init(&mut self) -> Result<(), Error> {
        // Initialization commands to send before
        // full communication can be established.
        // Without these, requests will always time out,
        // and the ECU wont understand what we're asking for.
        // Furthermore, causes unexpected behaviour when parsing the response.
        let commands = vec![
            Command::new_at(b"ATZ"),  // Reset all
            Command::new_at(b"ATE0"), // Echo off
            Command::new_at(b"ATL0"), // Linefeeds off
            Command::new_at(b"ATH1"), // Headers on
        ];

        for mut command in commands {
            // Cannot proceed with the initialization.
            // Refer to above. Furthermore, if we don't send a command
            // and read the buffer and then get junk values,
            // the program will be messed up.
            // Ensure the intiialization is 100% valid.

            let response = if self.replay_requests {
                self.get_recorded_response(&command)
            } else {
                self.send_command(&mut command)
                    .map_err(|_| Error::InitFailedSendingCommand(command.as_string()))?;
                self.get_at_response()?
            };

            if self.record_requests {
                self.save_request(&command, &response);
            }

            let acknowledged =
                validate_selection_ack(response.raw_response.as_deref().unwrap_or_default())
                    .is_ok();
            match (command.get_at(), response.formatted_response.as_deref()) {
                (b"ATZ", Some(data)) => {
                    self.elm_version = Some(data.to_owned());
                }
                (_, _) if acknowledged => {}
                x => {
                    println!("{:?}", x);
                    return Err(Error::InitFailedNoOKReceived);
                }
            }
        }

        Ok(())
    }

    /// Toggles whether PID requests are redirected to freeze frame (service 02).
    ///
    /// If `self.freeze_frame_query` is `true`, all service 01 PID requests
    /// will be redirected to service 02 (freeze frame).
    ///
    /// Returns the updated value of `freeze_frame_query`.
    pub fn query_freeze_frame(&mut self, state: bool) {
        self.freeze_frame_query = state
    }

    pub fn read_from_user_input(&mut self) {
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();
        loop {
            let mut input = String::new();

            let _ = stdout.write(b"\n> ");
            let _ = stdout.flush();
            let _ = stdin.read_line(&mut input);

            let message = input.replace("\n", "").replace("\r", "");
            if message == "exit" {
                break;
            }

            if self.send_command(&mut Command::new_arb(&message)).is_err() {
                println!("< error sending message {message}");
                continue;
            }

            if let Ok(response) = self.read_until(b'>') {
                if response.is_empty() {
                    continue;
                }

                let printable_raw = response.escape_default();
                println!("< '{printable_raw}'");
            } else {
                println!("< error receiving response for {message}");
            }
        }
    }

    pub fn send_command(&mut self, req: &mut Command) -> Result<(), Error> {
        // We don't need to send a command since we already
        // know what the respond will be.
        if self.replay_requests {
            return Ok(());
        }

        let stream = match &mut self.connection {
            Some(stream) => stream,
            None => return Err(Error::NoConnection),
        };

        let mut cmd = req.as_bytes();
        if cmd.is_empty() {
            return Ok(());
        }

        stream
            .set_timeout(ADAPTER_IDLE_TIMEOUT)
            .map_err(map_serial_error)?;
        stream
            .clear(serialport::ClearBuffer::All)
            .map_err(map_serial_error)?;

        cmd.push(b'\r');
        if let Err(error) = stream.write_all(&cmd) {
            self.connection = None;
            return Err(Error::AdapterReadWriteError(error.to_string()));
        }

        Ok(())
    }

    pub fn get_at_response(&mut self) -> Result<Response, Error> {
        let response = self.read_until(b'>')?;

        let meta_data = Response {
            raw_response: Some(response.clone()),
            formatted_response: Some(response.replace("\r", "")),

            ..Default::default()
        };

        Ok(meta_data)
    }

    pub fn get_pid_response(&mut self) -> Result<Response, Error> {
        let response = self.read_until(b'>')?;
        self.parse_pid_response(&response)
    }

    pub(crate) fn parse_pid_response(&self, raw_response: &str) -> Result<Response, Error> {
        let mut response = raw_response.replace("SEARCHING...", "").replace("\r", "\n");

        let payload_size = OBD::extract_payload_size(&response);
        let ecu_names = OBD::extract_ecu_names(&response);
        let escaped = response.clone();

        response = response.replace(" ", "").replace("\n", "").replacen(
            format!("{:02X}", payload_size).as_str(),
            "",
            1,
        );

        OBD::strip_ecu_names(&mut response, ecu_names.as_slice());

        if response.len() < 2 {
            return Err(Error::InvalidResponse);
        } else if response.contains("NODATA") {
            return Err(Error::NoData);
        }

        let parsed = Self::format_response(&response);
        let no_whitespace = parsed.replace(" ", "");
        let as_bytes = no_whitespace.as_bytes();

        let mut meta_data = Response::no_data();
        meta_data.responding_ecus = ecu_names;
        meta_data.formatted_response = Some(parsed.clone());
        meta_data.raw_response = Some(escaped);
        meta_data.payload_size = payload_size as usize;
        meta_data.service = [as_bytes[0], as_bytes[1]];
        meta_data.payload = Some(meta_data.payload_from_response());

        Ok(meta_data)
    }

    pub fn get_service_supported_pids(&mut self, service: &str) -> HashMap<String, Vec<String>> {
        // Service must be 2 characters long
        // An example of a service would be
        // '01' '05' or '09'
        if service.len() != 2 {
            println!("get_service_supported_pids; service ({service}) must be 2 characters long.");
            return HashMap::new();
        }

        let service_as_bytes = service.as_bytes();

        // A hashmap of supported pids for different ECUs
        // Key -> ECU Name
        // Value -> List of supported pids as strings
        let mut supported_pids: HashMap<String, Vec<String>> = HashMap::new();

        // The pids to request service 'service' to get a list of
        // supported pids by the car for that service
        let mut requests = Vec::new();

        if service == "01" {
            requests = vec!["00", "20", "40", "60", "80", "A0", "C0"];
        } else if service == "05" || service == "09" {
            requests = vec!["00"];
        }

        for request_pid in requests {
            let request_pid_bytes = request_pid.as_bytes();
            let command = [
                service_as_bytes[0],
                service_as_bytes[1],
                request_pid_bytes[0],
                request_pid_bytes[1],
            ];
            let response = self.query(Command::new_pid(&command));

            let split = format!("41{}", request_pid);

            let mut parsed: HashMap<String, Vec<String>> = self.parse_supported_pids(
                &response,
                &split,
                i32::from_str_radix(request_pid, 16).unwrap_or_default(),
            );

            for (ecu_name, pids) in parsed.iter_mut() {
                supported_pids
                    .entry(ecu_name.to_string())
                    .and_modify(|existing| existing.extend(pids.clone()))
                    .or_insert(pids.to_vec());
            }
        }
        supported_pids
    }

    // todo: inefficient...
    pub(crate) fn parse_supported_pids(
        &self,
        response: &Response,
        expected_header_split: &str,
        start_pid: i32,
    ) -> HashMap<String, Vec<String>> {
        let sanitized = response.get_payload().unwrap_or_default().replace(" ", "");
        let mut responses: Vec<&str> = sanitized.split(expected_header_split).collect();
        responses = responses
            .iter()
            .filter(|x| !x.is_empty())
            .copied()
            .collect();

        // println!("{:?}", responses);
        // println!("split: {expected_header_split}");

        let mut pid = start_pid + 1;
        let mut respective_pids = HashMap::new();

        // This is a loop because it is possible that
        // the vehicle returns multiple responses from
        // different ECUs, telling us what pids EACH ECU supports.
        for (index, data) in responses.iter().enumerate() {
            let ecu = match response.responding_ecus.get(index) {
                Some(ecu_name) => ecu_name.clone(),
                None => String::default(),
            };
            let mut supported_pids: Vec<String> = Vec::new();

            // Loop through all characters in 'data'
            // Get the character as a number from the hex character 'ch'
            for byte_str in data.as_bytes().chunks(2).flat_map(std::str::from_utf8) {
                let as_num = u8::from_str_radix(byte_str, 16).unwrap_or_default();

                // Iterate through each character in binary representation
                // If bit is 1, that is a supported pid.
                for i in 0..8 {
                    if (as_num & (1 << (7 - i))) != 0 {
                        supported_pids.push(format!("{:02X}", pid));
                    }
                    pid += 1;
                }
            }

            respective_pids.insert(ecu, supported_pids);
            pid = start_pid + 1;
        }

        respective_pids
    }

    pub fn extract_ecu_names(response: &str) -> Vec<String> {
        response
            .lines()
            .filter_map(|line| {
                let split: Vec<&str> = line.split_whitespace().collect();
                if split.len() < 3 {
                    return None;
                }

                let ecu_name = split[0].to_string();
                if ecu_name.len() != 3 {
                    // can ids are 3 character hex strings
                    return None;
                }

                Some(ecu_name)
            })
            .collect()
    }

    pub fn extract_payload_size(response: &str) -> u32 {
        response
            .split_whitespace()
            .nth(1)
            .and_then(|s| u32::from_str_radix(s, 16).ok())
            .unwrap_or(0)
    }

    pub(crate) fn strip_ecu_names(response: &mut String, ecu_names: &[String]) {
        for ecu_name in ecu_names.iter() {
            *response = response.replace(ecu_name, "")
        }
    }

    pub(crate) fn read_until(&mut self, until: u8) -> Result<String, Error> {
        if self.replay_requests {
            return Ok(String::new());
        }

        let port = match &mut self.connection {
            Some(port) => port,
            None => return Err(Error::NoConnection),
        };

        // The buffer is cleared before sending a command, never after it.
        // A fast adapter may already have returned the entire reply.
        let response = match read_serial_prompt(port.as_mut(), until) {
            Ok(response) => response,
            Err(error) => {
                // Without a validated prompt boundary, the next reply cannot be
                // attributed safely. Require an explicit reconnect; never retry.
                self.connection = None;
                return Err(error);
            }
        };

        if response.contains("UNABLE TO CONNECT") {
            println!("unable to connect");
            Err(Error::NoConnection)
        } else {
            Ok(response)
        }
    }

    pub(crate) fn read_iso_tp_response(&mut self) -> Vec<String> {
        // Reference table.
        // Example response that may have multiple frames:
        // (PID 0902 response):
        // 7E8 10 14 49 02 01 4D 41 54 - Frame 1
        // 7E8 21 34 30 33 30 39 36 42 - Frame 2
        // 7E8 22 4E 4C 30 30 30 30 30 - Frame 3
        //
        // 10 - First frame indicator ISO-TP
        // 14 - Total number of data bytes (20)
        // 49 - 4 + 09
        // 02 - response to 02 pid
        // 01 - record 01.
        // 4D 41 54 - payload starts
        // 21 - 1st consecutive frame
        // 22 - 2nd consecutive frame

        // Payload. Array of hex characters
        // (e.g ["4D", "41" ...])
        let mut payload = Vec::new();

        // Parsing
        let mut response = self.read_until(b'>').unwrap_or_default();
        response = response.replace("SEARCHING...", "");

        let ecu_names = OBD::extract_ecu_names(&response);
        OBD::strip_ecu_names(&mut response, ecu_names.as_slice());

        let stack_frames: Vec<&str> = response
            .split('\r')
            .filter(|l| !l.trim().is_empty())
            .collect();

        // Parse each stack frame

        println!("stack frames: {stack_frames:?}");
        println!();
        for frame in stack_frames {
            let clean = frame.trim();
            let bytes: Vec<&str> = clean.split_whitespace().collect();
            if bytes.len() < 2 {
                continue;
            }
            println!("Frame: {frame:?}");
            println!("Clean: {clean}");
            println!("Bytes: {bytes:?}");

            let pci = match u8::from_str_radix(bytes[0], 16) {
                Ok(b) => b,
                Err(_) => continue,
            };

            let frame_type = pci >> 4;

            match frame_type {
                0x0 => {
                    // single frame
                    let length = (pci & 0x0F) as usize;
                    if bytes.len() >= 2 + length {
                        payload.extend(bytes[2..2 + length].iter().map(|&s| s.to_string()));
                    }
                }
                // first frame
                0x1 if bytes.len() >= 4 => {
                    payload.extend(bytes[4..].iter().map(|&s| s.to_string()));
                }
                // consecutive frame
                0x2 if !bytes.is_empty() => {
                    payload.extend(bytes[1..].iter().map(|&s| s.to_string()));
                }
                _ => {}
            }
        }
        payload
    }

    pub fn get_vin(&mut self) -> Option<VIN> {
        match self.send_command(&mut Command::new_pid(b"0902")) {
            Ok(()) => (),
            Err(_) => return None,
        }

        // Convert hex payload to ASCII string
        let mut vin = String::new();
        for byte in self.read_iso_tp_response().iter().skip(1) {
            vin.push(
                u8::from_str_radix(byte, 16)
                    .map(|s| s as char)
                    .expect("call to u8::from_str_radix failed on str '{byte}'"),
            );
        }

        match VIN::new(&vin) {
            Ok(vin) => Some(vin),
            Err(err) => {
                println!("error when getting vin {vin}. {err}");
                None
            }
        }
    }

    pub(crate) fn format_response(response: &str) -> String {
        let chunks = response
            .as_bytes()
            .chunks(2)
            .map(|pair| std::str::from_utf8(pair).unwrap_or(""))
            .collect::<Vec<&str>>();

        chunks.join(" ")
    }

    pub fn query(&mut self, mut request: Command) -> Response {
        if self.freeze_frame_query && *request.command_type() == CommandType::PIDCommand {
            let pid = request.get_pid();
            if pid.starts_with(b"01") {
                request.set_pid(&[b'0', b'2', pid[2], pid[3]]);
            }
        }

        match self.send_command(&mut request) {
            Ok(_) => (),
            Err(err) => {
                println!(
                    "{}\tAT: '{}' - PID: '{}' ",
                    err,
                    String::from_utf8_lossy(request.get_at()),
                    String::from_utf8(request.get_pid().to_vec()).unwrap_or_default()
                );
                return Response::default();
            }
        };

        let response = if self.replay_requests {
            self.get_recorded_response(&request)
        } else {
            self.get_pid_response().unwrap_or(Response::no_data())
        };

        if self.record_requests {
            self.save_request(&request, &response);
        }

        response
    }

    pub fn set_unit_preferences(&mut self, preferences: UnitPreferences) {
        self.unit_preferences = preferences;
    }

    pub fn get_protocol_name(&mut self) -> Result<String, Error> {
        let mut request = Command::new_at(b"AT DP");
        self.send_command(&mut request)?;

        let response = if self.replay_requests {
            self.get_recorded_response(&request)
        } else {
            self.get_at_response().unwrap_or(Response::no_data())
        };

        if self.record_requests {
            self.save_request(&request, &response);
        }

        response.formatted_response.ok_or(Error::InvalidResponse)
    }

    /// Read the adapter's protocol configuration; this does not verify ECU communication.
    /// Demo recordings do not contain authoritative live adapter state.
    pub fn get_reported_protocol(&mut self) -> Result<ReportedProtocol, Error> {
        if self.replay_requests {
            return Err(Error::InvalidResponse);
        }
        let mut request = Command::new_at(b"AT DPN");
        self.send_command(&mut request)?;
        let response = self.get_at_response()?;
        parse_reported_protocol(response.raw_response.as_deref().unwrap_or_default())
            .map_err(|_| Error::InvalidResponse)
    }

    /// Read-only CAN probe, using the standard supported-PID request.
    /// A successful result correlates ECU response framing with the adapter report.
    /// The nominal bus speed is reported by the adapter, not measured independently.
    /// Legacy protocol layouts are not supported by this verification path.
    pub fn probe_can_protocol(&mut self) -> Result<ObdProtocol, Error> {
        if self.replay_requests {
            return Err(Error::InvalidResponse);
        }
        let selection = ProtocolSelection::try_from(self.protocol)
            .map_err(|_| Error::InvalidProtocol(self.protocol))?;
        if matches!(selection, ProtocolSelection::Manual(protocol) if protocol.can_parameters().is_none())
        {
            return Err(Error::InvalidResponse);
        }
        self.send_command(&mut Command::new_pid(b"0100"))?;
        let raw = self.read_until(b'>')?;
        can_probe_identifier_width(&raw).map_err(|_| Error::InvalidResponse)?;
        let report = self.get_reported_protocol()?;
        confirm_can_probe(selection, report, &raw).map_err(|_| Error::InvalidResponse)
    }

    /// Explicit read-only legacy probe. Only manually selected legacy protocols
    /// are accepted here; automatic detection is not yet orchestrated.
    /// Timing and initialization method are adapter-reported, not measured.
    pub fn probe_legacy_protocol(&mut self) -> Result<(ObdProtocol, Vec<LegacyProbe>), Error> {
        if self.replay_requests {
            return Err(Error::InvalidResponse);
        }
        let selection = ProtocolSelection::try_from(self.protocol)
            .map_err(|_| Error::InvalidProtocol(self.protocol))?;
        let ProtocolSelection::Manual(protocol) = selection else {
            return Err(Error::InvalidResponse);
        };
        if protocol.can_parameters().is_some() {
            return Err(Error::InvalidResponse);
        }

        self.send_command(&mut Command::new_pid(b"0100"))?;
        let raw = self.read_until(b'>')?;
        // Validate all ECU frames before requesting adapter configuration.
        let expected = ReportedProtocol {
            automatic: false,
            protocol: Some(protocol),
        };
        confirm_legacy_probe(selection, expected, &raw).map_err(|_| Error::InvalidResponse)?;
        let report = self.get_reported_protocol()?;
        confirm_legacy_probe(selection, report, &raw).map_err(|_| Error::InvalidResponse)
    }

    /// Requested configuration, not an independently verified ECU protocol.
    pub fn get_protocol_number(&self) -> u8 {
        if self.is_connected() {
            self.protocol
        } else {
            0
        }
    }

    /// Test and run Mode 22 pids from
    /// /data/model-pids.sqlite
    pub fn test_mode_22_pids(&mut self, vin: &VIN) {
        // This does not work when replaying requests
        if self.replay_requests {
            return;
        }

        // Get a Mode 22 pids for the model from the vin
        // Run them all, see if the output is valid.
        // Calculate equation and display
        // Sleep for a short period of time.
        // Repeat
        let model = match vin.get_engine_manufacturer() {
            Ok(em) => em,
            Err(_) => return,
        };

        // connect to mode 22 database
        let con = match sqlite::Connection::open(MODE22_PIDS_DB_PATH) {
            Ok(con) => con,
            Err(err) => {
                println!("when connecting to mode22 database: {err}");
                return;
            }
        };

        let query = "SELECT * FROM vehicle_pids WHERE model = ?";
        let mut statement = match con.prepare(query) {
            Ok(statement) => statement,
            Err(err) => {
                println!("when sanitizing statement {query}: {err}");
                return;
            }
        };

        match statement.bind((1, model.as_str())) {
            Ok(_) => {}
            Err(err) => {
                println!("when binding model '{}' to query {query}: {err}", model);
                return;
            }
        };

        while let Ok(State::Row) = statement.next() {
            let pid = statement
                .read::<String, _>("pid")
                .expect("reading column pid");
            let equation = statement
                .read::<String, _>("equation")
                .expect("reading column equation");
            let unit = statement
                .read::<String, _>("unit")
                .expect("reading column unit");
            let description = statement
                .read::<String, _>("description")
                .expect("reading description");

            let command = Command::new_arb(&pid);
            self.query(command).map_no_data(|response| {
                match self.calculate_dynamic_equation(&equation, &unit, &response) {
                    Ok(value) => {
                        println!("successfully calculated pid {pid}. equation: {equation}. unit {unit}");
                        println!("description: {description}");
                        println!("calculated: {value}");
                        value
                    },
                    Err(err) => {
                        println!("error trying to calculate pid {pid}. equation: {equation}. unit {unit}");
                        println!("description: {description}");
                        println!("error: {err}");
                        Scalar::no_data()
                    }
                }
            });

            sleep(Duration::from_millis(500));
        }
    }

    pub fn calculate_dynamic_equation(
        &mut self,
        equation: &str,
        unit: &str,
        response: &Response,
    ) -> Result<Scalar, Box<dyn std::error::Error>> {
        use evalexpr::*;

        if *response.get_payload_size() == 0 {
            return Ok(Scalar::no_data());
        }

        let equation = equation.to_uppercase();

        // TODO: fix this ugly code.
        let mut context: HashMapContext<DefaultNumericTypes> = HashMapContext::new();
        let variables = [
            ("A", response.a_value()),
            ("B", response.b_value()),
            ("C", response.c_value()),
            ("D", response.d_value()),
            ("E", response.e_value()),
        ];

        for (variable, value) in variables {
            if equation.contains(variable) {
                context.set_value(variable.into(), Value::from_float(value as f64))?;
            }
        }

        if equation.contains("A") {
            context.set_value("A".into(), Value::from_float(response.a_value() as f64))?;
        }
        if equation.contains("B") {
            context.set_value("B".into(), Value::from_float(response.b_value() as f64))?;
        }
        if equation.contains("C") {
            context.set_value("C".into(), Value::from_float(response.c_value() as f64))?;
        }
        if equation.contains("D") {
            context.set_value("D".into(), Value::from_float(response.d_value() as f64))?;
        }
        if equation.contains("E") {
            context.set_value("E".into(), Value::from_float(response.e_value() as f64))?;
        }

        match eval_float_with_context(&equation, &context) {
            Ok(res) => Ok(Scalar::new(
                res as f32,
                Unit::from_str(unit).unwrap_or(Unit::Unknown),
                Some(self.unit_preferences),
            )),
            Err(err) => {
                println!("when evaluating dynamic equation {equation}. {err}");
                Err(Box::new(err))
            }
        }
    }
}

#[cfg(test)]
mod serial_prompt_tests {
    use super::*;
    use std::io::{self, Cursor};

    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct FixtureState {
        events: Vec<&'static str>,
        pending: VecDeque<u8>,
        written: Vec<u8>,
        timeout: Duration,
        fail_timeout: bool,
        fail_clear: bool,
        fail_write: bool,
        eof_when_empty: bool,
        scripted_replies: VecDeque<Vec<u8>>,
    }

    #[derive(Clone)]
    struct FixturePort {
        state: Arc<Mutex<FixtureState>>,
    }

    impl SerialPort for FixturePort {
        fn name(&self) -> Option<String> {
            Some("fixture".to_string())
        }
        fn baud_rate(&self) -> serialport::Result<u32> {
            Ok(0)
        }
        fn data_bits(&self) -> serialport::Result<serialport::DataBits> {
            Ok(serialport::DataBits::Eight)
        }
        fn flow_control(&self) -> serialport::Result<serialport::FlowControl> {
            Ok(serialport::FlowControl::None)
        }
        fn parity(&self) -> serialport::Result<serialport::Parity> {
            Ok(serialport::Parity::None)
        }
        fn stop_bits(&self) -> serialport::Result<serialport::StopBits> {
            Ok(serialport::StopBits::One)
        }
        fn timeout(&self) -> Duration {
            self.state.lock().unwrap().timeout
        }
        fn set_timeout(&mut self, timeout: Duration) -> serialport::Result<()> {
            let mut state = self.state.lock().unwrap();
            state.events.push("timeout");
            if state.fail_timeout {
                return Err(serialport::Error::new(
                    serialport::ErrorKind::Unknown,
                    "fixture setup failure",
                ));
            }
            state.timeout = timeout;
            Ok(())
        }
        fn write_request_to_send(&mut self, _level: bool) -> serialport::Result<()> {
            Ok(())
        }
        fn write_data_terminal_ready(&mut self, _level: bool) -> serialport::Result<()> {
            Ok(())
        }
        fn read_clear_to_send(&mut self) -> serialport::Result<bool> {
            Ok(true)
        }
        fn read_data_set_ready(&mut self) -> serialport::Result<bool> {
            Ok(true)
        }
        fn read_ring_indicator(&mut self) -> serialport::Result<bool> {
            Ok(false)
        }
        fn read_carrier_detect(&mut self) -> serialport::Result<bool> {
            Ok(false)
        }
        fn bytes_to_read(&self) -> serialport::Result<u32> {
            Ok(0)
        }
        fn bytes_to_write(&self) -> serialport::Result<u32> {
            Ok(0)
        }
        fn clear(&self, _buffer: serialport::ClearBuffer) -> serialport::Result<()> {
            let mut state = self.state.lock().unwrap();
            state.events.push("clear");
            if state.fail_clear {
                return Err(serialport::Error::new(
                    serialport::ErrorKind::Unknown,
                    "fixture clear failure",
                ));
            }
            state.pending.clear();
            Ok(())
        }
        fn try_clone(&self) -> serialport::Result<Box<dyn SerialPort>> {
            Ok(Box::new(self.clone()))
        }
        fn set_baud_rate(&mut self, _baud_rate: u32) -> serialport::Result<()> {
            Ok(())
        }
        fn set_data_bits(&mut self, _data_bits: serialport::DataBits) -> serialport::Result<()> {
            Ok(())
        }
        fn set_flow_control(
            &mut self,
            _flow_control: serialport::FlowControl,
        ) -> serialport::Result<()> {
            Ok(())
        }
        fn set_parity(&mut self, _parity: serialport::Parity) -> serialport::Result<()> {
            Ok(())
        }
        fn set_stop_bits(&mut self, _stop_bits: serialport::StopBits) -> serialport::Result<()> {
            Ok(())
        }
        fn set_break(&self) -> serialport::Result<()> {
            Ok(())
        }
        fn clear_break(&self) -> serialport::Result<()> {
            Ok(())
        }
    }

    impl Read for FixturePort {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            let mut state = self.state.lock().unwrap();
            if buffer.is_empty() {
                return Ok(0);
            }
            if let Some(byte) = state.pending.pop_front() {
                buffer[0] = byte;
                Ok(1)
            } else if state.eof_when_empty {
                Ok(0)
            } else {
                Err(io::Error::new(io::ErrorKind::TimedOut, "fixture timeout"))
            }
        }
    }

    impl Write for FixturePort {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            let mut state = self.state.lock().unwrap();
            state.events.push("write");
            if state.fail_write {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "fixture write"));
            }
            state.written.extend_from_slice(buffer);
            // Simulate an adapter whose reply is already queued when write returns.
            let reply = state
                .scripted_replies
                .pop_front()
                .unwrap_or_else(|| b"ELM327 v1.5\r>".to_vec());
            state.pending.extend(reply);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn fixture_obd(state: FixtureState) -> (OBD, Arc<Mutex<FixtureState>>) {
        let state = Arc::new(Mutex::new(state));
        let mut obd = OBD::new();
        obd.connection = Some(Box::new(FixturePort {
            state: Arc::clone(&state),
        }));
        (obd, state)
    }

    fn initialization_replies(selection_reply: &[u8]) -> VecDeque<Vec<u8>> {
        VecDeque::from([
            b"ELM327 v1.5\r>".to_vec(),
            b"OK\r>".to_vec(),
            b"OK\r>".to_vec(),
            b"OK\r>".to_vec(),
            selection_reply.to_vec(),
        ])
    }

    #[test]
    fn every_protocol_selection_consumes_its_own_acknowledgement() {
        for id in 0..=9 {
            let (mut obd, state) = fixture_obd(FixtureState {
                scripted_replies: initialization_replies(b"OK\r>"),
                ..Default::default()
            });
            let selection = ProtocolSelection::try_from(id).unwrap();
            obd.initialize_protocol(selection, id).unwrap();
            assert_eq!(obd.get_protocol_number(), id);
            assert!(obd.is_connected());
            let state = state.lock().unwrap();
            let mut expected = b"ATZ\rATE0\rATL0\rATH1\r".to_vec();
            expected.extend_from_slice(selection.command());
            expected.push(b'\r');
            assert_eq!(state.written, expected);
            assert!(state.pending.is_empty());
        }
    }

    #[test]
    fn rejected_protocol_configuration_disconnects() {
        for reply in [b"?\r>".as_slice(), b"NOT OK\r>", b"OK\rERROR\r>"] {
            let (mut obd, _) = fixture_obd(FixtureState {
                scripted_replies: initialization_replies(reply),
                ..Default::default()
            });
            assert!(matches!(
                obd.initialize_protocol(ProtocolSelection::try_from(6).unwrap(), 6),
                Err(Error::InitFailedNoOKReceived)
            ));
            assert!(!obd.is_connected());
            assert_eq!(obd.get_protocol_number(), 0);
        }
    }

    #[test]
    fn failed_initialization_does_not_send_protocol_selection() {
        let (mut obd, state) = fixture_obd(FixtureState {
            scripted_replies: VecDeque::from([b"ELM327 v1.5\r>".to_vec(), b"?\r>".to_vec()]),
            ..Default::default()
        });
        assert!(obd
            .initialize_protocol(ProtocolSelection::try_from(6).unwrap(), 6)
            .is_err());
        assert!(!obd.is_connected());
        assert_eq!(state.lock().unwrap().written, b"ATZ\rATE0\r");
    }

    #[test]
    fn misleading_initialization_acknowledgements_stop_before_selection() {
        for reply in [b"NOT OK\r>".as_slice(), b"OK\rERROR\r>"] {
            let (mut obd, state) = fixture_obd(FixtureState {
                scripted_replies: VecDeque::from([b"ELM327 v1.5\r>".to_vec(), reply.to_vec()]),
                ..Default::default()
            });
            assert!(matches!(
                obd.initialize_protocol(ProtocolSelection::try_from(6).unwrap(), 6),
                Err(Error::InitFailedNoOKReceived)
            ));
            assert!(!obd.is_connected());
            assert_eq!(state.lock().unwrap().written, b"ATZ\rATE0\r");
        }
    }

    #[test]
    fn disconnect_clears_previous_protocol_configuration() {
        let (mut obd, _) = fixture_obd(FixtureState {
            scripted_replies: initialization_replies(b"OK\r>"),
            ..Default::default()
        });
        obd.initialize_protocol(ProtocolSelection::try_from(7).unwrap(), 7)
            .unwrap();
        obd.query_freeze_frame(true);
        obd.disconnect();
        assert_eq!(obd.get_protocol_number(), 0);
        assert!(obd.elm_version.is_none());
        assert!(!obd.freeze_frame_query);
    }

    #[test]
    fn reported_protocol_is_read_without_changing_requested_selection() {
        let (mut obd, state) = fixture_obd(FixtureState {
            scripted_replies: VecDeque::from([b"A7\r>".to_vec()]),
            ..Default::default()
        });
        let report = obd.get_reported_protocol().unwrap();
        assert!(report.automatic);
        assert_eq!(report.protocol.map(|protocol| protocol as u8), Some(7));
        assert_eq!(obd.get_protocol_number(), 0);
        assert_eq!(state.lock().unwrap().written, b"AT DPN\r");
    }

    #[test]
    fn can_probe_requires_positive_ecu_response_before_reading_adapter_report() {
        let (mut obd, state) = fixture_obd(FixtureState {
            scripted_replies: VecDeque::from([
                b"SEARCHING...\r18DAF110 06 41 00 00 00 00 00\r>".to_vec(),
                b"A9\r>".to_vec(),
            ]),
            ..Default::default()
        });
        assert_eq!(obd.probe_can_protocol().unwrap(), ObdProtocol::Can29At250);
        assert_eq!(state.lock().unwrap().written, b"0100\rAT DPN\r");
        assert_eq!(obd.get_protocol_number(), 0);
    }

    #[test]
    fn unsuccessful_can_probe_does_not_query_adapter_report() {
        let (mut obd, state) = fixture_obd(FixtureState {
            scripted_replies: VecDeque::from([b"NO DATA\r>".to_vec()]),
            ..Default::default()
        });
        assert!(matches!(
            obd.probe_can_protocol(),
            Err(Error::InvalidResponse)
        ));
        assert_eq!(state.lock().unwrap().written, b"0100\r");
    }

    #[test]
    fn manual_legacy_selection_does_not_start_can_probe() {
        let (mut obd, state) = fixture_obd(FixtureState::default());
        obd.protocol = 3;
        assert!(matches!(
            obd.probe_can_protocol(),
            Err(Error::InvalidResponse)
        ));
        assert!(state.lock().unwrap().written.is_empty());
    }

    #[test]
    fn can_probe_rejects_manual_protocol_mismatch() {
        let (mut obd, state) = fixture_obd(FixtureState {
            scripted_replies: VecDeque::from([
                b"7E8 06 41 00 FF FF FF FF\r>".to_vec(),
                b"8\r>".to_vec(),
            ]),
            ..Default::default()
        });
        obd.protocol = 6;
        assert!(matches!(
            obd.probe_can_protocol(),
            Err(Error::InvalidResponse)
        ));
        assert_eq!(state.lock().unwrap().written, b"0100\rAT DPN\r");
        assert_eq!(obd.get_protocol_number(), 6);
    }

    #[test]
    fn manual_legacy_probe_checks_all_five_protocols_through_serial_fixture() {
        for (id, reply) in [
            (1, "41 6B 10 41 00 00 00 00 00 9C"),
            (2, "48 6B 10 41 00 00 00 00 00 F6"),
            (3, "48 6B 10 41 00 00 00 00 00 04"),
            (4, "86 F1 11 41 00 FF FF FF FF C5"),
            (5, "86 F1 11 41 00 FF FF FF FF C5"),
        ] {
            let (mut obd, state) = fixture_obd(FixtureState {
                scripted_replies: VecDeque::from([
                    format!("{reply}\r>").into_bytes(),
                    format!("{id}\r>").into_bytes(),
                ]),
                ..Default::default()
            });
            obd.protocol = id;
            let (protocol, probes) = obd.probe_legacy_protocol().unwrap();
            assert_eq!(protocol, ObdProtocol::try_from(id).unwrap());
            assert_eq!(probes.len(), 1);
            assert_eq!(state.lock().unwrap().written, b"0100\rAT DPN\r");
            assert_eq!(obd.get_protocol_number(), id);
        }
    }

    #[test]
    fn invalid_legacy_frames_do_not_trigger_adapter_report() {
        for reply in [
            b"NO DATA\r>".as_slice(),
            b"48 6B 10 41 00 00 00 00 00 05\r>",
            b"48 6B 10 41 00 00 00 00 00 04\rNO DATA\r>",
        ] {
            let (mut obd, state) = fixture_obd(FixtureState {
                scripted_replies: VecDeque::from([reply.to_vec()]),
                ..Default::default()
            });
            obd.protocol = 3;
            assert!(matches!(
                obd.probe_legacy_protocol(),
                Err(Error::InvalidResponse)
            ));
            assert_eq!(state.lock().unwrap().written, b"0100\r");
        }
    }

    #[test]
    fn legacy_probe_rejects_report_mismatch_and_preserves_requested_selection() {
        for report in [b"2\r>".as_slice(), b"A3\r>", b"6\r>"] {
            let (mut obd, state) = fixture_obd(FixtureState {
                scripted_replies: VecDeque::from([
                    b"48 6B 10 41 00 00 00 00 00 04\r>".to_vec(),
                    report.to_vec(),
                ]),
                ..Default::default()
            });
            obd.protocol = 3;
            assert!(matches!(
                obd.probe_legacy_protocol(),
                Err(Error::InvalidResponse)
            ));
            assert_eq!(state.lock().unwrap().written, b"0100\rAT DPN\r");
            assert_eq!(obd.get_protocol_number(), 3);
        }
    }

    #[test]
    fn legacy_probe_does_not_dispatch_in_automatic_can_or_replay_mode() {
        for id in [0, 6, 7, 8, 9] {
            let (mut obd, state) = fixture_obd(FixtureState::default());
            obd.protocol = id;
            assert!(matches!(
                obd.probe_legacy_protocol(),
                Err(Error::InvalidResponse)
            ));
            assert!(state.lock().unwrap().written.is_empty());
        }
        let (mut obd, state) = fixture_obd(FixtureState::default());
        obd.protocol = 3;
        obd.replay_requests = true;
        assert!(matches!(
            obd.probe_legacy_protocol(),
            Err(Error::InvalidResponse)
        ));
        assert!(state.lock().unwrap().written.is_empty());
    }

    #[test]
    fn transaction_keeps_reply_queued_during_write() {
        let (mut obd, state) = fixture_obd(FixtureState {
            pending: VecDeque::from(*b"X"),
            ..Default::default()
        });

        obd.send_command(&mut Command::new_at(b"ATI"))
            .expect("send succeeds");
        let reply = obd.get_at_response().expect("receive succeeds");

        assert_eq!(reply.raw_response().as_deref(), Some("ELM327 v1.5\r"));
        let state = state.lock().unwrap();
        assert_eq!(state.written, b"ATI\r");
        assert_eq!(state.events, ["timeout", "clear", "write"]);
        assert!(state.pending.is_empty());
    }

    #[test]
    fn timeout_setup_failure_prevents_write() {
        let (mut obd, state) = fixture_obd(FixtureState {
            fail_timeout: true,
            ..Default::default()
        });

        assert!(matches!(
            obd.send_command(&mut Command::new_at(b"ATI")),
            Err(Error::AdapterReadWriteError(_))
        ));
        let state = state.lock().unwrap();
        assert_eq!(state.events, ["timeout"]);
        assert!(state.written.is_empty());
    }

    #[test]
    fn clear_failure_prevents_write() {
        let (mut obd, state) = fixture_obd(FixtureState {
            fail_clear: true,
            ..Default::default()
        });

        assert!(matches!(
            obd.send_command(&mut Command::new_at(b"ATI")),
            Err(Error::AdapterReadWriteError(_))
        ));
        let state = state.lock().unwrap();
        assert_eq!(state.events, ["timeout", "clear"]);
        assert!(state.written.is_empty());
    }

    #[test]
    fn failed_write_is_not_success_and_disconnects() {
        let (mut obd, state) = fixture_obd(FixtureState {
            fail_write: true,
            ..Default::default()
        });

        assert!(matches!(
            obd.send_command(&mut Command::new_at(b"ATI")),
            Err(Error::AdapterReadWriteError(_))
        ));
        assert!(!obd.is_connected());
        assert!(matches!(
            obd.send_command(&mut Command::new_at(b"ATI")),
            Err(Error::NoConnection)
        ));
        let state = state.lock().unwrap();
        assert_eq!(state.events, ["timeout", "clear", "write"]);
        assert!(state.written.is_empty());
    }

    #[test]
    fn partial_reply_timeout_invalidates_connection_and_blocks_next_write() {
        let (mut obd, state) = fixture_obd(FixtureState::default());
        obd.send_command(&mut Command::new_at(b"ATI")).unwrap();
        state.lock().unwrap().pending = VecDeque::from(*b"ELM");
        assert!(matches!(
            obd.get_at_response(),
            Err(Error::AdapterReadWriteError(_))
        ));
        assert!(!obd.is_connected());

        // Simulate the late remainder arriving after the failed transaction.
        state.lock().unwrap().pending.extend(b"327\r>");
        assert!(matches!(
            obd.send_command(&mut Command::new_at(b"ATI")),
            Err(Error::NoConnection)
        ));
        let state = state.lock().unwrap();
        assert_eq!(state.written, b"ATI\r");
        assert_eq!(state.events, ["timeout", "clear", "write"]);
    }

    #[test]
    fn premature_eof_invalidates_connection() {
        let (mut obd, state) = fixture_obd(FixtureState {
            eof_when_empty: true,
            ..Default::default()
        });
        obd.send_command(&mut Command::new_at(b"ATI")).unwrap();
        state.lock().unwrap().pending = VecDeque::from(*b"ELM");
        assert!(matches!(obd.get_at_response(), Err(Error::InvalidResponse)));
        assert!(!obd.is_connected());
    }

    #[test]
    fn malformed_byte_before_prompt_invalidates_connection() {
        let (mut obd, state) = fixture_obd(FixtureState::default());
        obd.send_command(&mut Command::new_at(b"ATI")).unwrap();
        state.lock().unwrap().pending = VecDeque::from([b'E', 0xff, b'>']);
        assert!(matches!(obd.get_at_response(), Err(Error::InvalidResponse)));
        assert!(!obd.is_connected());
        assert_eq!(state.lock().unwrap().written, b"ATI\r");
    }

    #[test]
    fn complete_no_data_reply_keeps_transport_synchronized() {
        let (mut obd, state) = fixture_obd(FixtureState::default());
        obd.send_command(&mut Command::new_pid(b"010C")).unwrap();
        state.lock().unwrap().pending = VecDeque::from(*b"NO DATA\r>");
        assert!(matches!(obd.get_pid_response(), Err(Error::NoData)));
        assert!(obd.is_connected());
        obd.send_command(&mut Command::new_at(b"ATI")).unwrap();
        assert!(obd.get_at_response().is_ok());
        assert_eq!(state.lock().unwrap().written, b"010C\rATI\r");
    }

    #[test]
    fn immediate_reply_is_preserved_until_prompt() {
        let mut reader = Cursor::new(b"41 0C 1A F8\r>next");
        let result = read_prompt(&mut reader, b'>').expect("complete reply");
        assert_eq!(result, "41 0C 1A F8\r");
        assert_eq!(reader.position(), b"41 0C 1A F8\r>".len() as u64);
    }

    #[test]
    fn incomplete_reply_is_not_success() {
        let mut reader = Cursor::new(b"41 0C");
        assert!(matches!(
            read_prompt(&mut reader, b'>'),
            Err(Error::InvalidResponse)
        ));
    }

    #[test]
    fn io_error_after_partial_data_is_propagated() {
        struct BrokenReader {
            first: bool,
        }
        impl Read for BrokenReader {
            fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
                if self.first {
                    self.first = false;
                    buffer[0] = b'4';
                    Ok(1)
                } else {
                    Err(io::Error::new(io::ErrorKind::TimedOut, "fixture timeout"))
                }
            }
        }

        let mut reader = BrokenReader { first: true };
        assert!(matches!(
            read_prompt(&mut reader, b'>'),
            Err(Error::AdapterReadWriteError(_))
        ));
    }

    #[test]
    fn reply_size_limit_is_enforced() {
        let mut bytes = vec![b'A'; MAX_ADAPTER_REPLY_BYTES];
        bytes.push(b'>');
        let mut reader = Cursor::new(bytes);
        let reply = read_prompt(&mut reader, b'>').expect("maximum complete reply");
        assert_eq!(reply.len(), MAX_ADAPTER_REPLY_BYTES);

        let mut reader = Cursor::new(vec![b'A'; MAX_ADAPTER_REPLY_BYTES + 1]);
        assert!(matches!(
            read_prompt(&mut reader, b'>'),
            Err(Error::InvalidResponse)
        ));
    }

    #[test]
    fn continuously_arriving_bytes_cannot_extend_total_deadline() {
        use std::cell::Cell;
        let elapsed = Cell::new(Duration::ZERO);
        let reads = Cell::new(0);
        let result = read_prompt_with_deadline(
            b'>',
            || elapsed.get(),
            |buffer, timeout| {
                assert!(timeout <= ADAPTER_IDLE_TIMEOUT);
                elapsed.set(elapsed.get() + Duration::from_millis(500));
                reads.set(reads.get() + 1);
                buffer[0] = b'A';
                Ok(1)
            },
        );
        assert!(matches!(result, Err(Error::ReplyDeadlineExceeded)));
        assert_eq!(reads.get(), 10);
    }

    #[test]
    fn final_read_budget_is_clamped_and_late_prompt_is_rejected() {
        use std::cell::Cell;
        let elapsed = Cell::new(ADAPTER_REPLY_DEADLINE - Duration::from_millis(200));
        let result = read_prompt_with_deadline(
            b'>',
            || elapsed.get(),
            |buffer, timeout| {
                assert_eq!(timeout, Duration::from_millis(200));
                elapsed.set(ADAPTER_REPLY_DEADLINE);
                buffer[0] = b'>';
                Ok(1)
            },
        );
        assert!(matches!(result, Err(Error::ReplyDeadlineExceeded)));
    }

    #[test]
    fn prompt_before_deadline_is_accepted() {
        use std::cell::Cell;
        let elapsed = Cell::new(ADAPTER_REPLY_DEADLINE - Duration::from_millis(200));
        let result = read_prompt_with_deadline(
            b'>',
            || elapsed.get(),
            |buffer, timeout| {
                assert_eq!(timeout, Duration::from_millis(200));
                elapsed.set(ADAPTER_REPLY_DEADLINE - Duration::from_millis(1));
                buffer[0] = b'>';
                Ok(1)
            },
        );
        assert_eq!(result.unwrap(), "");
    }

    #[test]
    fn expired_budget_never_starts_another_read() {
        let result = read_prompt_with_deadline(
            b'>',
            || ADAPTER_REPLY_DEADLINE,
            |_, _| panic!("read must not begin after deadline"),
        );
        assert!(matches!(result, Err(Error::ReplyDeadlineExceeded)));
    }

    #[test]
    fn non_ascii_reply_is_rejected() {
        let mut reader = Cursor::new([b'4', 0xff, b'>']);
        assert!(matches!(
            read_prompt(&mut reader, b'>'),
            Err(Error::InvalidResponse)
        ));
    }
}
