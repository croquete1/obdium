use std::fmt;

use crate::{
    scalar::{Scalar, Unit},
    Command, SensorNumber, Service, OBD,
};

#[derive(Clone, Copy, Debug)]
pub enum CylinderNumber {
    Cylinder1,
    Cylinder2,
    Cylinder3,
    Cylinder4,
    Cylinder5,
    Cylinder6,
    Cylinder7,
    Cylinder8,
    Cylinder9,
    Cylinder10,
    Cylinder11,
    Cylinder12,
}

#[derive(PartialEq, Eq)]
pub enum EngineType {
    SparkIgnition,
    CompressionIgnition,
    Unknown,
}

impl EngineType {
    pub fn as_str(&self) -> &str {
        match self {
            EngineType::SparkIgnition => "Internal Combustion Engine",
            EngineType::CompressionIgnition => "Compression Ignition Engine",
            EngineType::Unknown => "Unkown",
        }
    }
}

impl fmt::Display for EngineType {
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

fn rpm_from_response(
    response: &crate::Response,
    preferences: crate::scalar::UnitPreferences,
) -> Scalar {
    match response.checked_pid_data::<2>(&[0x41, 0x0C]) {
        Ok(bytes) => Scalar::new(
            f32::from(u16::from_be_bytes(bytes)) / 4.0,
            Unit::RPM,
            Some(preferences),
        ),
        Err(_) => Scalar::no_data(),
    }
}

impl OBD {
    pub fn rpm(&mut self) -> Scalar {
        let response = self.query(Command::new_pid(b"010C"));
        rpm_from_response(&response, self.unit_preferences)
    }

    pub fn engine_load(&mut self) -> Scalar {
        self.query(Command::new_pid(b"0104"))
            .map_checked_pid_data::<1>(&[0x41, 0x04], |[a]| {
                Scalar::new(
                    f32::from(a) / 2.55,
                    Unit::Percent,
                    Some(self.unit_preferences),
                )
            })
    }

    pub fn coolant_temp(&mut self) -> Scalar {
        self.query(Command::new_pid(b"0105"))
            .map_checked_pid_data::<1>(&[0x41, 0x05], |[a]| {
                Scalar::new(
                    f32::from(a) - 40.0,
                    Unit::Celsius,
                    Some(self.unit_preferences),
                )
            })
    }

    pub fn coolant_temp_sensors(&mut self) -> (Scalar, Scalar) {
        let mut coolant_temp = (Scalar::no_data(), Scalar::no_data());
        let response = self.query(Command::new_pid(b"0167"));
        if *response.get_payload_size() == 0 {
            return coolant_temp;
        }

        let sensors_supported = self.sensors_supported_for(response.a_value() as u8);

        if sensors_supported.contains(&SensorNumber::Sensor1) {
            coolant_temp.0 = Scalar::new(
                response.b_value() - 40.0,
                Unit::Celsius,
                Some(self.unit_preferences),
            )
        }

        if sensors_supported.contains(&SensorNumber::Sensor2) {
            coolant_temp.1 = Scalar::new(
                response.c_value() - 40.0,
                Unit::Celsius,
                Some(self.unit_preferences),
            );
        }

        coolant_temp
    }

    pub fn engine_fuel_rate(&mut self) -> Scalar {
        self.query(Command::new_pid(b"019D")).map_no_data(|r| {
            Scalar::new(
                r.a_value(),
                Unit::GramsPerSecond,
                Some(self.unit_preferences),
            )
        })
    }

    pub fn engine_runtime(&mut self) -> Scalar {
        if self.get_engine_type() == EngineType::CompressionIgnition {
            return self.engine_runtime_diesel();
        }

        self.query(Command::new_pid(b"011F")).map_no_data(|r| {
            Scalar::new(
                (256.0 * r.a_value()) + r.b_value(),
                Unit::Seconds,
                Some(self.unit_preferences),
            )
        })
    }

    fn engine_runtime_diesel(&mut self) -> Scalar {
        let response = self.query(Command::new_pid(b"017F"));
        if *response.get_payload_size() == 0 {
            return Scalar::no_data();
        }

        let b = response.b_value();
        let c = response.c_value();
        let d = response.d_value();
        let e = response.e_value();
        let b_power = f32::powf(2f32, 24f32);
        let c_power = f32::powf(2f32, 16f32);
        let d_power = f32::powf(2f32, 8f32);

        Scalar::new(
            (b * b_power) + (c * c_power) + (d * d_power) + e,
            Unit::Seconds,
            Some(self.unit_preferences),
        )
    }

    pub fn odometer(&mut self) -> Scalar {
        let response = self.query(Command::new_pid(b"01A6"));
        if *response.get_payload_size() == 0 {
            return Scalar::no_data();
        }

        let a = response.a_value();
        let b = response.b_value();
        let c = response.c_value();
        let d = response.d_value();

        let a_power = f32::powf(2f32, 24f32);
        let b_power = f32::powf(2f32, 16f32);
        let c_power = f32::powf(2f32, 8f32);

        Scalar::new(
            ((a * a_power) + (b * b_power) + (c * c_power) + d) / 10.0,
            Unit::Kilometers,
            Some(self.unit_preferences),
        )
    }

    pub fn engine_oil_temp(&mut self, mode: Service) -> Scalar {
        let command = match mode {
            Service::Mode01 => Command::new_pid(b"015C"),
            Service::Mode22 => Command::new_arb("221154"),
        };

        self.query(command).map_no_data(|r| {
            Scalar::new(
                r.a_value() - 40.0,
                Unit::Celsius,
                Some(self.unit_preferences),
            )
        })
    }

    pub fn engine_oil_temp_sensors(&mut self) -> (Scalar, Scalar) {
        let mut oil_temp = (Scalar::no_data(), Scalar::no_data());
        let response = self.query(Command::new_pid(b"0167"));
        if *response.get_payload_size() == 0 {
            return oil_temp;
        }

        let sensors_supported = self.sensors_supported_for(response.a_value() as u8);

        if sensors_supported.contains(&SensorNumber::Sensor1) {
            oil_temp.0 = Scalar::new(
                response.b_value() - 40.0,
                Unit::Celsius,
                Some(self.unit_preferences),
            )
        }

        if sensors_supported.contains(&SensorNumber::Sensor2) {
            oil_temp.1 = Scalar::new(
                response.c_value() - 40.0,
                Unit::Celsius,
                Some(self.unit_preferences),
            );
        }

        oil_temp
    }

    pub fn engine_oil_pressure(&mut self) -> Scalar {
        self.query(Command::new_arb("221470")).map_no_data(|r| {
            Scalar::new(
                r.a_value() * 3.985,
                Unit::KiloPascal,
                Some(self.unit_preferences),
            )
        })
    }

    pub fn drivers_demand_engine_torque(&mut self) -> Scalar {
        self.query(Command::new_pid(b"0161")).map_no_data(|r| {
            Scalar::new(
                r.a_value() - 125.0,
                Unit::Percent,
                Some(self.unit_preferences),
            )
        })
    }

    pub fn actual_engine_torque(&mut self) -> Scalar {
        self.query(Command::new_pid(b"0162")).map_no_data(|r| {
            Scalar::new(
                r.a_value() - 125.0,
                Unit::Percent,
                Some(self.unit_preferences),
            )
        })
    }

    pub fn reference_engine_torque(&mut self) -> Scalar {
        self.query(Command::new_pid(b"0163")).map_no_data(|r| {
            Scalar::new(
                (256.0 * r.a_value()) + r.b_value(),
                Unit::NewtonMeters,
                Some(self.unit_preferences),
            )
        })
    }

    pub fn engine_oil_life(&mut self) -> Scalar {
        self.query(Command::new_arb("221940"))
            .map_no_data(|r| Scalar::new(r.a_value(), Unit::Percent, None))
    }

    // Returns 5 values.
    // idle - torque at idle
    // engine_point_1 - torque at engine point 1
    // engine_point_2 - torque at engine point 2
    // engine_point_3 - torque at engine point 3
    // engine_point_4 - torque at engine point 4
    pub fn engine_percent_torque_data(&mut self) -> (Scalar, Scalar, Scalar, Scalar, Scalar) {
        let response = self.query(Command::new_pid(b"0164"));
        if *response.get_payload_size() == 0 {
            return (
                Scalar::no_data(),
                Scalar::no_data(),
                Scalar::no_data(),
                Scalar::no_data(),
                Scalar::no_data(),
            );
        }

        let idle = response.a_value() - 125.0;
        let engine_point_1 = response.b_value() - 125.0;
        let engine_point_2 = response.c_value() - 125.0;
        let engine_point_3 = response.d_value() - 125.0;
        let engine_point_4 = response.e_value() - 125.0;

        (
            Scalar::new(idle, Unit::Percent, Some(self.unit_preferences)),
            Scalar::new(engine_point_1, Unit::Percent, Some(self.unit_preferences)),
            Scalar::new(engine_point_2, Unit::Percent, Some(self.unit_preferences)),
            Scalar::new(engine_point_3, Unit::Percent, Some(self.unit_preferences)),
            Scalar::new(engine_point_4, Unit::Percent, Some(self.unit_preferences)),
        )
    }

    pub fn get_engine_type(&mut self) -> EngineType {
        let response = self.query(Command::new_pid(b"0101"));
        if *response.get_payload_size() == 0 {
            return EngineType::Unknown;
        }

        match response.b_value() as u32 & 0b00001000 {
            0 => EngineType::SparkIgnition,
            1 => EngineType::CompressionIgnition,
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod checked_rpm_tests {
    use super::*;
    use crate::Response;

    fn fixture(payload: &str) -> Response {
        Response {
            payload: Some(payload.into()),
            payload_size: 4,
            ..Default::default()
        }
    }

    #[test]
    fn actual_zero_rpm_is_distinct_from_missing_data() {
        let zero = rpm_from_response(&fixture("41 0C 00 00"), Default::default());
        assert_eq!(zero.value, 0.0);
        assert_eq!(zero.unit, Unit::RPM);
        let missing = rpm_from_response(&Response::no_data(), Default::default());
        assert_eq!(missing.unit, Unit::NoData);
    }

    #[test]
    fn rpm_scaling_uses_both_bytes() {
        let rpm = rpm_from_response(&fixture("41 0C 1A F8"), Default::default());
        assert_eq!(rpm.value, 1726.0);
        assert_eq!(rpm.unit, Unit::RPM);
    }

    #[test]
    fn incomplete_or_invalid_rpm_is_not_a_measurement() {
        for payload in [
            "41 0C 1A",
            "41 0C GG F8",
            "41 0C +1 F8",
            "41 0C é F8",
            "41 0D 1A F8",
            "42 0C 1A F8",
            "41 0C 1A F8 41 0C 00 00",
        ] {
            let result = rpm_from_response(&fixture(payload), Default::default());
            assert_eq!(result.unit, Unit::NoData, "payload: {payload}");
        }
    }
}
