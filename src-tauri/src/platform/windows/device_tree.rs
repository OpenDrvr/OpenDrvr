//! Windows Plug and Play enumeration through the Configuration Manager API.
//!
//! CfgMgr32 is the primary source of device identity, status, and properties.
//! SetupAPI is reserved for future driver-package/INF inspection, and WMI/CIM
//! is supplemental for facts that are not exposed as PnP device properties.
//!
//! Every property read is optional. A missing property is normal; an unexpected
//! failure is recorded in [`ScanDiagnostics`] and the device record is emitted
//! with that field unset. Only failure to obtain the device ID list is fatal.

use std::fmt;
use std::time::Instant;

use windows::core::{GUID, PCWSTR};
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CM_Get_DevNode_PropertyW, CM_Get_DevNode_Status, CM_Get_Device_ID_ListW,
    CM_Get_Device_ID_List_SizeW, CM_Locate_DevNodeW, CM_DEVNODE_STATUS_FLAGS,
    CM_GETIDLIST_FILTER_PRESENT, CM_LOCATE_DEVNODE_NORMAL, CM_PROB, CR_BUFFER_SMALL,
    CR_NO_SUCH_VALUE, CR_SUCCESS,
};
use windows::Win32::Devices::Properties::{
    DEVPKEY_Device_Address, DEVPKEY_Device_BusNumber, DEVPKEY_Device_Class,
    DEVPKEY_Device_ClassGuid, DEVPKEY_Device_CompatibleIds, DEVPKEY_Device_DeviceDesc,
    DEVPKEY_Device_DriverDate, DEVPKEY_Device_DriverDesc, DEVPKEY_Device_DriverInfPath,
    DEVPKEY_Device_DriverProvider, DEVPKEY_Device_DriverVersion, DEVPKEY_Device_EnumeratorName,
    DEVPKEY_Device_FriendlyName, DEVPKEY_Device_HardwareIds, DEVPKEY_Device_IsPresent,
    DEVPKEY_Device_LocationInfo, DEVPKEY_Device_LocationPaths, DEVPKEY_Device_Manufacturer,
    DEVPKEY_Device_Parent, DEVPKEY_Device_Service, DEVPROPTYPE, DEVPROP_TYPE_BOOLEAN,
    DEVPROP_TYPE_FILETIME, DEVPROP_TYPE_GUID, DEVPROP_TYPE_STRING, DEVPROP_TYPE_STRING_LIST,
    DEVPROP_TYPE_UINT32,
};
use windows::Win32::Foundation::DEVPROPKEY;

use crate::hardware::{
    category::DeviceCategory,
    id::{DeviceInstanceId, HardwareId},
    model::{
        DeviceFailure, DeviceLocation, DeviceRecord, DriverInfo, HardwareSnapshot, PropertyFailure,
        ScanDiagnostics,
    },
    provider::{DeviceEnumerator, HardwareError},
    status::DeviceStatus,
};

/// CfgMgr32-backed enumerator for the local machine.
#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsDeviceEnumerator;

impl DeviceEnumerator for WindowsDeviceEnumerator {
    fn enumerate(&self) -> Result<HardwareSnapshot, HardwareError> {
        enumerate_present_devices()
    }
}

fn enumerate_present_devices() -> Result<HardwareSnapshot, HardwareError> {
    let started = Instant::now();
    let instance_ids = device_instance_ids()?;

    let mut diagnostics = ScanDiagnostics {
        provider: "CfgMgr32".to_string(),
        present_only: true,
        ..ScanDiagnostics::default()
    };
    let mut devices = Vec::with_capacity(instance_ids.len());

    for raw_instance_id in instance_ids {
        let Some(instance_id) = DeviceInstanceId::new(raw_instance_id.clone()) else {
            diagnostics.device_failures.push(DeviceFailure {
                instance_id: raw_instance_id,
                operation: "DeviceInstanceId::new".to_string(),
                error: "empty or NUL-containing instance ID".to_string(),
            });
            continue;
        };

        if let Some(device) = enumerate_device(&instance_id, &mut diagnostics) {
            devices.push(device);
        }
    }

    devices.sort_by(|left, right| left.instance_id.cmp(&right.instance_id));
    diagnostics.device_count = devices.len();
    diagnostics.duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);

    Ok(HardwareSnapshot {
        devices,
        diagnostics,
    })
}

fn device_instance_ids() -> Result<Vec<String>, HardwareError> {
    let filter = PCWSTR::null();
    let mut last_code = 0;

    for _ in 0..3 {
        let mut size = 0u32;
        let result =
            unsafe { CM_Get_Device_ID_List_SizeW(&mut size, filter, CM_GETIDLIST_FILTER_PRESENT) };

        if result != CR_SUCCESS {
            return Err(HardwareError::ConfigManager {
                operation: "CM_Get_Device_ID_List_SizeW",
                code: result.0,
            });
        }
        if size == 0 {
            return Ok(Vec::new());
        }

        let mut buffer = vec![0u16; size as usize];
        let result =
            unsafe { CM_Get_Device_ID_ListW(filter, &mut buffer, CM_GETIDLIST_FILTER_PRESENT) };

        if result == CR_SUCCESS {
            return Ok(parse_device_id_list(&buffer));
        }
        if result != CR_BUFFER_SMALL {
            return Err(HardwareError::ConfigManager {
                operation: "CM_Get_Device_ID_ListW",
                code: result.0,
            });
        }

        last_code = result.0;
    }

    Err(HardwareError::ConfigManager {
        operation: "CM_Get_Device_ID_ListW",
        code: last_code,
    })
}

fn parse_device_id_list(buffer: &[u16]) -> Vec<String> {
    let mut identifiers = Vec::new();
    let mut start = 0;

    for (index, &unit) in buffer.iter().enumerate() {
        if unit == 0 {
            if index > start {
                identifiers.push(String::from_utf16_lossy(&buffer[start..index]));
            }
            start = index + 1;
        }
    }

    if start < buffer.len() && buffer[start..].iter().any(|&unit| unit != 0) {
        let end = buffer[start..]
            .iter()
            .position(|&unit| unit == 0)
            .map(|offset| start + offset)
            .unwrap_or(buffer.len());
        identifiers.push(String::from_utf16_lossy(&buffer[start..end]));
    }

    identifiers
}

fn enumerate_device(
    instance_id: &DeviceInstanceId,
    diagnostics: &mut ScanDiagnostics,
) -> Option<DeviceRecord> {
    let devinst = match locate_devnode(instance_id) {
        Ok(devinst) => devinst,
        Err(code) => {
            diagnostics.device_failures.push(DeviceFailure {
                instance_id: instance_id.as_str().to_string(),
                operation: "CM_Locate_DevNodeW".to_string(),
                error: format!("Configuration Manager error {code}"),
            });
            return None;
        }
    };

    let status = match devnode_status(devinst) {
        Ok(status) => status,
        Err(code) => {
            diagnostics.device_failures.push(DeviceFailure {
                instance_id: instance_id.as_str().to_string(),
                operation: "CM_Get_DevNode_Status".to_string(),
                error: format!("Configuration Manager error {code}"),
            });
            DeviceStatus::unknown()
        }
    };

    let class_name = property_optional(
        diagnostics,
        instance_id,
        "DEVPKEY_Device_Class",
        read_string(devinst, &DEVPKEY_Device_Class),
    );
    let class_guid = property_optional(
        diagnostics,
        instance_id,
        "DEVPKEY_Device_ClassGuid",
        read_guid(devinst, &DEVPKEY_Device_ClassGuid),
    )
    .map(|guid| format_guid(&guid));
    let manufacturer = property_optional(
        diagnostics,
        instance_id,
        "DEVPKEY_Device_Manufacturer",
        read_string(devinst, &DEVPKEY_Device_Manufacturer),
    );
    let friendly_name = property_optional(
        diagnostics,
        instance_id,
        "DEVPKEY_Device_FriendlyName",
        read_string(devinst, &DEVPKEY_Device_FriendlyName),
    );
    let device_description = property_optional(
        diagnostics,
        instance_id,
        "DEVPKEY_Device_DeviceDesc",
        read_string(devinst, &DEVPKEY_Device_DeviceDesc),
    );
    let enumerator = property_optional(
        diagnostics,
        instance_id,
        "DEVPKEY_Device_EnumeratorName",
        read_string(devinst, &DEVPKEY_Device_EnumeratorName),
    )
    .or_else(|| instance_id.enumerator().map(ToOwned::to_owned));
    let parent_instance_id = property_optional(
        diagnostics,
        instance_id,
        "DEVPKEY_Device_Parent",
        read_string(devinst, &DEVPKEY_Device_Parent),
    )
    .and_then(DeviceInstanceId::new);

    let hardware_ids = property_optional(
        diagnostics,
        instance_id,
        "DEVPKEY_Device_HardwareIds",
        read_string_list(devinst, &DEVPKEY_Device_HardwareIds),
    )
    .unwrap_or_default()
    .into_iter()
    .map(HardwareId::parse)
    .collect::<Vec<_>>();
    let compatible_ids = property_optional(
        diagnostics,
        instance_id,
        "DEVPKEY_Device_CompatibleIds",
        read_string_list(devinst, &DEVPKEY_Device_CompatibleIds),
    )
    .unwrap_or_default()
    .into_iter()
    .map(HardwareId::parse)
    .collect::<Vec<_>>();

    let location_info = property_optional(
        diagnostics,
        instance_id,
        "DEVPKEY_Device_LocationInfo",
        read_string(devinst, &DEVPKEY_Device_LocationInfo),
    );
    let location_paths = property_optional(
        diagnostics,
        instance_id,
        "DEVPKEY_Device_LocationPaths",
        read_string_list(devinst, &DEVPKEY_Device_LocationPaths),
    )
    .unwrap_or_default();
    let bus_number = property_optional(
        diagnostics,
        instance_id,
        "DEVPKEY_Device_BusNumber",
        read_u32(devinst, &DEVPKEY_Device_BusNumber),
    );
    let address = property_optional(
        diagnostics,
        instance_id,
        "DEVPKEY_Device_Address",
        read_u32(devinst, &DEVPKEY_Device_Address),
    );
    let is_present = property_optional(
        diagnostics,
        instance_id,
        "DEVPKEY_Device_IsPresent",
        read_bool(devinst, &DEVPKEY_Device_IsPresent),
    )
    .unwrap_or(true);

    let driver_provider = property_optional(
        diagnostics,
        instance_id,
        "DEVPKEY_Device_DriverProvider",
        read_string(devinst, &DEVPKEY_Device_DriverProvider),
    );
    let driver_version = property_optional(
        diagnostics,
        instance_id,
        "DEVPKEY_Device_DriverVersion",
        read_string(devinst, &DEVPKEY_Device_DriverVersion),
    );
    let driver_date = property_optional(
        diagnostics,
        instance_id,
        "DEVPKEY_Device_DriverDate",
        read_filetime(devinst, &DEVPKEY_Device_DriverDate),
    )
    .and_then(format_filetime);
    let driver_description = property_optional(
        diagnostics,
        instance_id,
        "DEVPKEY_Device_DriverDesc",
        read_string(devinst, &DEVPKEY_Device_DriverDesc),
    );
    let driver_inf_path = property_optional(
        diagnostics,
        instance_id,
        "DEVPKEY_Device_DriverInfPath",
        read_string(devinst, &DEVPKEY_Device_DriverInfPath),
    );
    let driver_service = property_optional(
        diagnostics,
        instance_id,
        "DEVPKEY_Device_Service",
        read_string(devinst, &DEVPKEY_Device_Service),
    );

    let has_driver_metadata = driver_provider.is_some()
        || driver_version.is_some()
        || driver_date.is_some()
        || driver_description.is_some()
        || driver_inf_path.is_some()
        || driver_service.is_some();
    let driver = has_driver_metadata.then_some(DriverInfo {
        provider: driver_provider,
        version: driver_version,
        date: driver_date,
        description: driver_description,
        inf_path: driver_inf_path,
        service: driver_service,
    });

    let bus_type = instance_id.bus_type();
    let category = DeviceCategory::classify(
        class_name.as_deref(),
        &bus_type,
        &hardware_ids,
        &compatible_ids,
    );

    Some(DeviceRecord {
        instance_id: instance_id.clone(),
        hardware_ids,
        compatible_ids,
        bus_type,
        category,
        class_name,
        class_guid,
        manufacturer,
        friendly_name,
        device_description,
        enumerator,
        parent_instance_id,
        location: DeviceLocation {
            location_info,
            location_paths,
            bus_number,
            address,
        },
        driver,
        status,
        is_present,
    })
}

fn locate_devnode(instance_id: &DeviceInstanceId) -> Result<u32, u32> {
    let wide = wide_string(instance_id.as_str());
    let mut devinst = 0u32;
    let result = unsafe {
        CM_Locate_DevNodeW(
            &mut devinst,
            PCWSTR::from_raw(wide.as_ptr()),
            CM_LOCATE_DEVNODE_NORMAL,
        )
    };

    if result == CR_SUCCESS {
        Ok(devinst)
    } else {
        Err(result.0)
    }
}

fn devnode_status(devinst: u32) -> Result<DeviceStatus, u32> {
    let mut flags = CM_DEVNODE_STATUS_FLAGS::default();
    let mut problem = CM_PROB::default();
    let result = unsafe { CM_Get_DevNode_Status(&mut flags, &mut problem, devinst, 0) };

    if result == CR_SUCCESS {
        Ok(DeviceStatus::from_flags(flags.0, problem.0))
    } else {
        Err(result.0)
    }
}

fn property_optional<T>(
    diagnostics: &mut ScanDiagnostics,
    instance_id: &DeviceInstanceId,
    property: &'static str,
    result: Result<Option<T>, PropertyError>,
) -> Option<T> {
    match result {
        Ok(value) => value,
        Err(error) => {
            diagnostics.property_failures.push(PropertyFailure {
                instance_id: instance_id.clone(),
                property: property.to_string(),
                error: error.to_string(),
            });
            None
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum PropertyError {
    Config(u32),
    UnexpectedType,
    Malformed,
}

impl fmt::Display for PropertyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(code) => write!(formatter, "Configuration Manager error {code}"),
            Self::UnexpectedType => formatter.write_str("unexpected property type"),
            Self::Malformed => formatter.write_str("malformed property payload"),
        }
    }
}

fn read_property(
    devinst: u32,
    key: &DEVPROPKEY,
) -> Result<Option<(DEVPROPTYPE, Vec<u8>)>, PropertyError> {
    let mut property_type = DEVPROPTYPE::default();
    let mut size = 0u32;
    let result =
        unsafe { CM_Get_DevNode_PropertyW(devinst, key, &mut property_type, None, &mut size, 0) };

    if result == CR_NO_SUCH_VALUE {
        return Ok(None);
    }
    if result != CR_SUCCESS && result != CR_BUFFER_SMALL {
        return Err(PropertyError::Config(result.0));
    }
    if size == 0 {
        return Ok(None);
    }

    let mut buffer = vec![0u8; size as usize];
    let result = unsafe {
        CM_Get_DevNode_PropertyW(
            devinst,
            key,
            &mut property_type,
            Some(buffer.as_mut_ptr()),
            &mut size,
            0,
        )
    };

    if result == CR_NO_SUCH_VALUE {
        return Ok(None);
    }
    if result != CR_SUCCESS {
        return Err(PropertyError::Config(result.0));
    }
    if size as usize > buffer.len() {
        return Err(PropertyError::Malformed);
    }

    buffer.truncate(size as usize);
    Ok(Some((property_type, buffer)))
}

fn read_string(devinst: u32, key: &DEVPROPKEY) -> Result<Option<String>, PropertyError> {
    let Some((property_type, bytes)) = read_property(devinst, key)? else {
        return Ok(None);
    };
    if property_type != DEVPROP_TYPE_STRING {
        return Err(PropertyError::UnexpectedType);
    }

    let value = decode_string(&bytes)?;
    Ok((!value.is_empty()).then_some(value))
}

fn read_string_list(devinst: u32, key: &DEVPROPKEY) -> Result<Option<Vec<String>>, PropertyError> {
    let Some((property_type, bytes)) = read_property(devinst, key)? else {
        return Ok(None);
    };
    if property_type != DEVPROP_TYPE_STRING_LIST {
        return Err(PropertyError::UnexpectedType);
    }

    decode_multi_string(&bytes).map(Some)
}

fn read_u32(devinst: u32, key: &DEVPROPKEY) -> Result<Option<u32>, PropertyError> {
    let Some((property_type, bytes)) = read_property(devinst, key)? else {
        return Ok(None);
    };
    if property_type != DEVPROP_TYPE_UINT32 {
        return Err(PropertyError::UnexpectedType);
    }

    let bytes: [u8; 4] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| PropertyError::Malformed)?;
    Ok(Some(u32::from_le_bytes(bytes)))
}

fn read_bool(devinst: u32, key: &DEVPROPKEY) -> Result<Option<bool>, PropertyError> {
    let Some((property_type, bytes)) = read_property(devinst, key)? else {
        return Ok(None);
    };
    if property_type != DEVPROP_TYPE_BOOLEAN {
        return Err(PropertyError::UnexpectedType);
    }

    let value = *bytes.first().ok_or(PropertyError::Malformed)?;
    Ok(Some(value != 0))
}

fn read_guid(devinst: u32, key: &DEVPROPKEY) -> Result<Option<GUID>, PropertyError> {
    let Some((property_type, bytes)) = read_property(devinst, key)? else {
        return Ok(None);
    };
    if property_type != DEVPROP_TYPE_GUID {
        return Err(PropertyError::UnexpectedType);
    }
    if bytes.len() != std::mem::size_of::<GUID>() {
        return Err(PropertyError::Malformed);
    }

    // The property buffer is byte-aligned, so read the GUID unaligned.
    let guid = unsafe { std::ptr::read_unaligned(bytes.as_ptr().cast::<GUID>()) };
    Ok(Some(guid))
}

fn read_filetime(devinst: u32, key: &DEVPROPKEY) -> Result<Option<u64>, PropertyError> {
    let Some((property_type, bytes)) = read_property(devinst, key)? else {
        return Ok(None);
    };
    if property_type != DEVPROP_TYPE_FILETIME {
        return Err(PropertyError::UnexpectedType);
    }

    let bytes: [u8; 8] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| PropertyError::Malformed)?;
    Ok(Some(u64::from_le_bytes(bytes)))
}

fn decode_string(bytes: &[u8]) -> Result<String, PropertyError> {
    let units = utf16_units(bytes)?;
    let end = units
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(units.len());
    Ok(String::from_utf16_lossy(&units[..end]))
}

fn decode_multi_string(bytes: &[u8]) -> Result<Vec<String>, PropertyError> {
    let units = utf16_units(bytes)?;
    let mut values = Vec::new();
    let mut start = 0;

    for (index, &unit) in units.iter().enumerate() {
        if unit == 0 {
            if index > start {
                values.push(String::from_utf16_lossy(&units[start..index]));
            }
            start = index + 1;
        }
    }

    if start < units.len() && units[start..].iter().any(|&unit| unit != 0) {
        let end = units[start..]
            .iter()
            .position(|&unit| unit == 0)
            .map(|offset| start + offset)
            .unwrap_or(units.len());
        values.push(String::from_utf16_lossy(&units[start..end]));
    }

    Ok(values)
}

fn utf16_units(bytes: &[u8]) -> Result<Vec<u16>, PropertyError> {
    if !bytes.len().is_multiple_of(2) {
        return Err(PropertyError::Malformed);
    }

    Ok(bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|chunk| u16::from_le_bytes(*chunk))
        .collect())
}

fn wide_string(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn format_guid(guid: &GUID) -> String {
    format!(
        "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        guid.data1,
        guid.data2,
        guid.data3,
        guid.data4[0],
        guid.data4[1],
        guid.data4[2],
        guid.data4[3],
        guid.data4[4],
        guid.data4[5],
        guid.data4[6],
        guid.data4[7]
    )
}

fn format_filetime(filetime: u64) -> Option<String> {
    const TICKS_PER_SECOND: u64 = 10_000_000;
    const SECONDS_BETWEEN_1601_AND_1970: u64 = 11_644_473_600;

    let seconds_since_1601 = filetime / TICKS_PER_SECOND;
    let unix_seconds = seconds_since_1601.checked_sub(SECONDS_BETWEEN_1601_AND_1970)?;
    let days = (unix_seconds / 86_400) as i64;
    let seconds_of_day = unix_seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;

    Some(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z"
    ))
}

/// Converts days since the Unix epoch to a civil date (Howard Hinnant's
/// `civil_from_days` algorithm).
fn civil_from_days(days_since_unix_epoch: i64) -> (i64, u32, u32) {
    let z = days_since_unix_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    };

    (year + i64::from(month <= 2), month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_multi_sz_device_id_list() {
        let buffer: Vec<u16> =
            "PCI\\VEN_8086&DEV_1234\\3&11583659&0&10\0USB\\VID_046D&PID_C52B\\5&2A1B2C3D&0&1\0\0"
                .encode_utf16()
                .collect();

        assert_eq!(
            parse_device_id_list(&buffer),
            vec![
                "PCI\\VEN_8086&DEV_1234\\3&11583659&0&10".to_string(),
                "USB\\VID_046D&PID_C52B\\5&2A1B2C3D&0&1".to_string()
            ]
        );
    }

    #[test]
    fn decodes_string_property_payloads() {
        let string_bytes: Vec<u8> = "Intel\0"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        let list_bytes: Vec<u8> = "PCI\\VEN_8086&DEV_1234\0USB\\VID_046D&PID_C52B\0\0"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();

        assert_eq!(decode_string(&string_bytes).expect("string"), "Intel");
        assert_eq!(
            decode_multi_string(&list_bytes).expect("string list"),
            vec![
                "PCI\\VEN_8086&DEV_1234".to_string(),
                "USB\\VID_046D&PID_C52B".to_string()
            ]
        );
    }

    #[test]
    fn formats_guid_in_canonical_form() {
        let guid = GUID::from_u128(0x4d36e968_e325_11ce_bfc1_08002be10318);

        assert_eq!(format_guid(&guid), "4d36e968-e325-11ce-bfc1-08002be10318");
    }

    #[test]
    fn formats_filetime_as_utc() {
        assert_eq!(
            format_filetime(132_223_104_000_000_000).as_deref(),
            Some("2020-01-01T00:00:00Z")
        );
        assert_eq!(format_filetime(0), None);
    }

    #[test]
    fn enumerates_present_devices_from_the_local_machine() {
        let snapshot = WindowsDeviceEnumerator
            .enumerate()
            .expect("enumerate present devices");

        assert!(snapshot.diagnostics.present_only);
        assert_eq!(snapshot.diagnostics.provider, "CfgMgr32");
        assert_eq!(snapshot.diagnostics.device_count, snapshot.devices.len());
        assert!(!snapshot.devices.is_empty());
        assert!(snapshot.devices.iter().all(|device| device.is_present));
        assert!(snapshot
            .devices
            .iter()
            .any(|device| !device.hardware_ids.is_empty()));
    }
}
