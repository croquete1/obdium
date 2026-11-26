use std::collections::HashMap;

use crate::{engine::CylinderNumber, BankNumber, SensorNumber, OBD};

pub enum EvapLeakSize {
    Large,  // 0.150"
    Medium, // 0.090"
    Small,  // 0.040"
    Tiny,   // 0.020"
}

#[derive(Debug)]
pub struct MonitorTest {
    /// Min and maximum allowed values for the specific test
    /// If a value is less than `min` or greater than `max`, the test is considered a FAIL.
    min: u64,
    max: u64,

    value: u64,
    mid: [u8; 2],
}

impl MonitorTest {
    /// Identifier of the decoded monitor test.
    pub fn mid(&self) -> [u8; 2] {
        self.mid
    }

    pub fn has_passed(&self) -> bool {
        self.value >= self.min && self.value <= self.max
    }
}

/// Mode 06 is not implemented yet; no diagnostic result is available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("Mode 06 monitor tests are not implemented")]
pub struct Mode06Unavailable;

impl OBD {
    pub fn get_supported_mids(
        &mut self,
    ) -> Result<HashMap<String, Vec<String>>, Mode06Unavailable> {
        Err(Mode06Unavailable)
    }

    pub fn test_oxygen_sensor_monitor(
        &mut self,
        _bank: BankNumber,
        _sensor: SensorNumber,
    ) -> Result<MonitorTest, Mode06Unavailable> {
        Err(Mode06Unavailable)
    }

    pub fn test_catalyst_monitor(
        &mut self,
        _bank: BankNumber,
    ) -> Result<MonitorTest, Mode06Unavailable> {
        Err(Mode06Unavailable)
    }

    // Exhaust gas recirculation
    pub fn test_egr_monitor(
        &mut self,
        _bank: BankNumber,
    ) -> Result<MonitorTest, Mode06Unavailable> {
        Err(Mode06Unavailable)
    }

    // Variable valve timing
    pub fn test_vvt_monitor(
        &mut self,
        _bank: BankNumber,
    ) -> Result<MonitorTest, Mode06Unavailable> {
        Err(Mode06Unavailable)
    }

    pub fn test_evap_monitor(
        &mut self,
        _leak_size: EvapLeakSize,
    ) -> Result<MonitorTest, Mode06Unavailable> {
        Err(Mode06Unavailable)
    }

    pub fn test_purge_flow_monitor(&mut self) -> Result<MonitorTest, Mode06Unavailable> {
        Err(Mode06Unavailable)
    }

    pub fn test_oxygen_sensor_heater(
        &mut self,
        _bank: BankNumber,
        _sensor: SensorNumber,
    ) -> Result<MonitorTest, Mode06Unavailable> {
        Err(Mode06Unavailable)
    }

    pub fn test_heated_catalyst_monitor(
        &mut self,
        _bank: BankNumber,
    ) -> Result<MonitorTest, Mode06Unavailable> {
        Err(Mode06Unavailable)
    }

    pub fn test_secondary_air_monitor(
        &mut self,
        _id: u8,
    ) -> Result<MonitorTest, Mode06Unavailable> {
        Err(Mode06Unavailable)
    }

    pub fn test_fuel_system_monitor(
        &mut self,
        _bank: BankNumber,
    ) -> Result<MonitorTest, Mode06Unavailable> {
        Err(Mode06Unavailable)
    }

    pub fn test_boost_pressure_control_monitor(
        &mut self,
        _bank: BankNumber,
    ) -> Result<MonitorTest, Mode06Unavailable> {
        Err(Mode06Unavailable)
    }

    pub fn test_nox_absorber_monitor(
        &mut self,
        _bank: BankNumber,
    ) -> Result<MonitorTest, Mode06Unavailable> {
        Err(Mode06Unavailable)
    }

    pub fn test_nox_catalyst_monitor(
        &mut self,
        _bank: BankNumber,
    ) -> Result<MonitorTest, Mode06Unavailable> {
        Err(Mode06Unavailable)
    }

    pub fn test_misfire_monitor_general(&mut self) -> Result<MonitorTest, Mode06Unavailable> {
        Err(Mode06Unavailable)
    }

    pub fn test_misfire_cylinder_monitor(
        &mut self,
        _cylinder: CylinderNumber,
    ) -> Result<MonitorTest, Mode06Unavailable> {
        Err(Mode06Unavailable)
    }

    pub fn test_pm_filter_monitor(
        &mut self,
        _bank: BankNumber,
    ) -> Result<MonitorTest, Mode06Unavailable> {
        Err(Mode06Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_mode06_never_reports_success_or_panics() {
        let mut obd = OBD::new();
        assert!(matches!(obd.get_supported_mids(), Err(Mode06Unavailable)));
        assert!(matches!(
            obd.test_oxygen_sensor_monitor(BankNumber::Bank1, SensorNumber::Sensor1),
            Err(Mode06Unavailable)
        ));
        assert!(matches!(
            obd.test_catalyst_monitor(BankNumber::Bank1),
            Err(Mode06Unavailable)
        ));
        assert!(matches!(
            obd.test_egr_monitor(BankNumber::Bank1),
            Err(Mode06Unavailable)
        ));
        assert!(matches!(
            obd.test_vvt_monitor(BankNumber::Bank1),
            Err(Mode06Unavailable)
        ));
        assert!(matches!(
            obd.test_evap_monitor(EvapLeakSize::Small),
            Err(Mode06Unavailable)
        ));
        assert!(matches!(
            obd.test_purge_flow_monitor(),
            Err(Mode06Unavailable)
        ));
        assert!(matches!(
            obd.test_oxygen_sensor_heater(BankNumber::Bank1, SensorNumber::Sensor1),
            Err(Mode06Unavailable)
        ));
        assert!(matches!(
            obd.test_heated_catalyst_monitor(BankNumber::Bank1),
            Err(Mode06Unavailable)
        ));
        assert!(matches!(
            obd.test_secondary_air_monitor(1),
            Err(Mode06Unavailable)
        ));
        assert!(matches!(
            obd.test_fuel_system_monitor(BankNumber::Bank1),
            Err(Mode06Unavailable)
        ));
        assert!(matches!(
            obd.test_boost_pressure_control_monitor(BankNumber::Bank1),
            Err(Mode06Unavailable)
        ));
        assert!(matches!(
            obd.test_nox_absorber_monitor(BankNumber::Bank1),
            Err(Mode06Unavailable)
        ));
        assert!(matches!(
            obd.test_nox_catalyst_monitor(BankNumber::Bank1),
            Err(Mode06Unavailable)
        ));
        assert!(matches!(
            obd.test_misfire_monitor_general(),
            Err(Mode06Unavailable)
        ));
        assert!(matches!(
            obd.test_misfire_cylinder_monitor(CylinderNumber::Cylinder1),
            Err(Mode06Unavailable)
        ));
        assert!(matches!(
            obd.test_pm_filter_monitor(BankNumber::Bank1),
            Err(Mode06Unavailable)
        ));
    }

    #[test]
    fn monitor_limits_are_inclusive() {
        for (value, expected) in [(9, false), (10, true), (15, true), (20, true), (21, false)] {
            let result = MonitorTest {
                min: 10,
                max: 20,
                value,
                mid: [1, 2],
            };
            assert_eq!(result.has_passed(), expected);
            assert_eq!(result.mid(), [1, 2]);
        }
    }
}
