# Windows hardware integration

This document describes the native hardware layer implemented in **Phase 10**. It
records the Windows API assessment, the portable data model, the provider
architecture, identifier parsing and vendor resolution, privilege and
compatibility expectations, and the boundary before driver catalog matching.

Phase 10 exposes a read-only scan through the `scan_hardware` command and the
Hardware page. It parses PCI, USB, ACPI, and HD Audio identifiers, resolves known
vendors, and classifies devices. It does not persist scans, match devices against
the catalog, download drivers, or install anything.

## Goals and boundaries

The scanner identifies at least:

- CPU and GPU devices
- motherboard and chipset devices
- PCI and USB devices
- Ethernet, Wi-Fi, and Bluetooth devices
- audio devices
- storage devices and controllers
- input devices
- devices Windows cannot classify
- devices with missing, failed, or blocked drivers

Identity comes from Plug and Play data, not from friendly names. The raw device
instance ID and hardware IDs are authoritative. PCI, USB, ACPI, and HD Audio
parsing is a convenience for matching and presentation; an unrecognized or
malformed string is always preserved verbatim. A curated vendor table resolves
known vendor and subsystem vendor IDs, and every record carries a confidence of
`confirmed`, `partial`, or `unresolved`.

Phase 10 does **not**:

- persist scans or scan history
- match devices against the catalog
- invent manufacturer or model names
- treat Windows-reported names as confirmed identity
- query WMI/CIM
- inspect or install driver packages
- download drivers
- request administrator elevation

## Windows API assessment

| API                                                   | Role                                              | Strengths                                                                                                                                                                    | Limits                                                                                                                             | Phase 10 decision                                                  |
| ----------------------------------------------------- | ------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| Configuration Manager (`cfgmgr32.dll`)                | Primary PnP device tree access                    | Machine-wide enumeration without class GUIDs; device instance IDs; status flags and problem codes; typed device properties; parent/child relationships; notification support | Lower-level than SetupAPI; driver-package/INF details are limited; some properties are unavailable to standard users               | **Used now** for enumeration, status, and device/driver properties |
| SetupAPI (`setupapi.dll`)                             | Device sets, registry properties, driver packages | Class/group enumeration; `SetupDiGetDeviceRegistryProperty`; driver info lists; INF inspection; installation APIs                                                            | Requires an `HDEVINFO` set and often a class GUID; older property model; driver-store/install operations need administrator rights | **Reserved** for future driver-package and INF inspection          |
| PnP manager / device installation APIs (`newdev.dll`) | Driver installation and updates                   | `DiInstallDriver`, `DiInstallDevice`, `DiUninstallDevice`, `UpdateDriverForPlugAndPlayDevices`                                                                               | Administrator-only, changes system state, needs rollback and signature checks                                                      | **Out of scope** until a later driver-install phase                |
| WMI/CIM (`root\cimv2`)                                | Supplemental system facts                         | `Win32_Processor`, `Win32_VideoController`, `Win32_BaseBoard`, `Win32_BIOS`, `Win32_PhysicalMemory`, `Win32_NetworkAdapter`, `Win32_PnPEntity`, `Win32_PnPSignedDriver`      | Slower; COM initialization; some classes need administrator rights; not the authoritative PnP identity                             | **Reserved** for supplemental CPU/GPU/board/BIOS/memory facts      |

### Why CfgMgr32 is primary

CfgMgr32 exposes the device tree directly and does not require the caller to
enumerate class GUIDs first. It provides:

- `CM_Get_Device_ID_List_SizeW` and `CM_Get_Device_ID_ListW` to list device
  instance IDs, with `CM_GETIDLIST_FILTER_NONE` for all installed nodes and
  `CM_GETIDLIST_FILTER_PRESENT` available when only the current tree is needed
- `CM_Locate_DevNodeW` to obtain a device node handle, with
  `CM_LOCATE_DEVNODE_PHANTOM` for installed but non-present devices
- `CM_Get_DevNode_Status` for `DN_*` flags and `CM_PROB_*` problem codes
- `CM_Get_DevNode_PropertyW` for typed `DEVPKEY_Device_*` properties
- `CM_Get_Parent`, `CM_Get_Child`, and `CM_Get_Sibling` for tree traversal if a
  future view needs hierarchy beyond the stored parent ID

SetupAPI remains valuable later because it is the documented path for driver
information lists and INF/package inspection. Keeping the provider boundary
narrow means the scanner can add SetupAPI without changing the portable model.

### Property keys used in Phase 10

| Area           | Property                                                                                                                                                                            |
| -------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Identity       | `DEVPKEY_Device_HardwareIds`, `DEVPKEY_Device_CompatibleIds`, `DEVPKEY_Device_InstanceId` (via the list)                                                                            |
| Classification | `DEVPKEY_Device_Class`, `DEVPKEY_Device_ClassGuid`, `DEVPKEY_Device_EnumeratorName`                                                                                                 |
| Description    | `DEVPKEY_Device_FriendlyName`, `DEVPKEY_Device_DeviceDesc`, `DEVPKEY_Device_Manufacturer`                                                                                           |
| Topology       | `DEVPKEY_Device_Parent`, `DEVPKEY_Device_LocationInfo`, `DEVPKEY_Device_LocationPaths`, `DEVPKEY_Device_BusNumber`, `DEVPKEY_Device_Address`                                        |
| Presence       | `DEVPKEY_Device_IsPresent`                                                                                                                                                          |
| Driver         | `DEVPKEY_Device_DriverProvider`, `DEVPKEY_Device_DriverVersion`, `DEVPKEY_Device_DriverDate`, `DEVPKEY_Device_DriverDesc`, `DEVPKEY_Device_DriverInfPath`, `DEVPKEY_Device_Service` |
| Status         | `CM_Get_DevNode_Status` (`DN_*` flags, `CM_PROB_*`)                                                                                                                                 |

`DEVPKEY_Device_InstanceId` is not read separately because the instance ID is
the key returned by the list call.

## Architecture

```text
React useHardwareScan
  └── invokeCommand("scan_hardware")
        └── commands::hardware::scan_hardware
              └── spawn_blocking
                    └── HardwareService
                          └── Arc<dyn DeviceEnumerator>        portable hardware::provider trait
                                └── WindowsDeviceEnumerator    platform::windows::device_tree
                                      └── CfgMgr32             cfgmgr32.dll through windows-rs
```

- `hardware/` owns the portable model, identifier parsing, vendor tables,
  identification, status mapping, categorization, and the provider trait. It
  never imports Win32 or Tauri.
- `platform::device_enumerator()` constructs the platform provider. `AppState`
  injects it into `HardwareService`.
- `platform::windows::device_tree` is the only module that calls CfgMgr32. It
  returns the portable `HardwareSnapshot`.
- The dependency is inverted: the hardware domain does not depend on platform
  code. A future test or alternate provider can implement `DeviceEnumerator`
  without a Windows device tree.

`HardwareService::scan()` runs on a blocking worker because Configuration
Manager calls are synchronous. The module status is `Ready`. Scans are returned
directly to the caller and are not persisted.

### Enumeration flow

1. Query the required UTF-16 buffer size with
   `CM_Get_Device_ID_List_SizeW(NULL, CM_GETIDLIST_FILTER_NONE)`.
2. Read the multi-string device ID list with `CM_Get_Device_ID_ListW`. A
   `CR_BUFFER_SMALL` result is retried a bounded number of times because the
   device tree can change between the size query and the read.
3. Deduplicate instance IDs case-insensitively while preserving Windows order.
4. For each instance ID, locate the device node with
   `CM_Locate_DevNodeW(CM_LOCATE_DEVNODE_NORMAL)`. If that fails, retry with
   `CM_LOCATE_DEVNODE_PHANTOM` so installed but disconnected devices are still
   reported.
5. Read presence first, then status and problem code with
   `CM_Get_DevNode_Status`.
6. Read each optional property with a size query followed by a typed read.
   Missing values (`CR_NO_SUCH_VALUE`) are normal and are not diagnostics.
7. Derive `DeviceCondition` from presence, status, problem kind, and driver
   metadata.
8. Resolve `DeviceIdentification` from the instance ID, hardware and compatible
   IDs, and Windows-reported manufacturer and name.
9. Emit one `DeviceRecord` per device, sort by instance ID, and deduplicate once
   more for deterministic output.

Only failure to obtain the device ID list is fatal. A device that cannot be
located, or whose status cannot be read, is recorded in diagnostics while the
rest of the scan continues. Property failures for non-present devices are not
treated as warnings because those devices are expected to expose fewer
properties. A property with an unexpected type or malformed payload is recorded
as a property failure and left unset.

## Portable data model

### Identity

`DeviceInstanceId` wraps the exact Windows string, for example:

```text
PCI\VEN_8086&DEV_A70D&SUBSYS_12345678&REV_01\3&11583659&0&10
USB\VID_046D&PID_C52B\5&2A1B2C3D&0&1
```

It rejects empty or NUL-containing strings, exposes the enumerator prefix, and
derives a best-effort `BusType`. The string itself is never normalized for
identity.

`HardwareId` stores:

- `raw` — the exact hardware or compatible ID
- `kind` — `Pci`, `Usb`, `Acpi`, `Hdaudio`, or `Unknown`

PCI parsing recognizes `VEN`, `DEV`, `SUBSYS`, `REV`, and `CC` tokens. In
`SUBSYS_ssssdddd`, the first four hex digits are the subsystem device ID and the
last four are the subsystem vendor ID. USB parsing recognizes `VID`, `PID`,
`REV`, `MI`, `CLASS`, `SUBCLASS`, and `PROT`. ACPI parsing supports both
`VEN_vvvv&DEV_dddd` and compact EISA-style IDs such as `PNP0A08`, `INT3450`, and
`ACPI0003`. HD Audio parsing recognizes `FUNC`, `VEN`, `DEV`, `SUBSYS`, and
`REV`; unlike PCI, `SUBSYS_ssssdddd` lists the subsystem vendor first and the
subsystem device second. Malformed fields become `None` without changing `raw`.

### Identification

`hardware/vendors.rs` maps public vendor identifiers to names through curated
PCI, USB, ACPI, and HD Audio tables. The tables are intentionally a useful subset,
not an exhaustive database: an unknown ID stays unresolved. Adding a vendor or a
new identifier family is a change to this module only. ACPI specification
namespace prefixes such as `PNP`, `ACPI`, and `MSFT` are not treated as
manufacturers because the namespace owner is not necessarily the device vendor.

`hardware/identify.rs` produces `DeviceIdentification`:

- `confidence`: `confirmed`, `partial`, or `unresolved`
- `manufacturer`: preferred identity, using the subsystem vendor when available
  and otherwise the hardware vendor
- `hardware_vendor`: silicon/device vendor from `VEN_` or `VID_`
- `subsystem_vendor`: board/system vendor from a subsystem field, when present
- `reported_manufacturer`: the exact Windows `DEVPKEY_Device_Manufacturer` value
- `reported_name`: the Windows friendly name or description
- `identifier_families`: identifier families discovered on the device
- `notes`: explanations for partial, unresolved, or conflicting identity

A result is `confirmed` only when a known device vendor was resolved and the
Windows-reported manufacturer does not contradict it. Subsystem-only identity,
incomplete identifiers, and manufacturer conflicts are `partial`. When no known
vendor ID matches, the result is `unresolved`; reported values are still shown but
never promoted to confirmed identity. The driver provider remains a separate
field because it is not necessarily the device manufacturer.

### Status

`DeviceStatus` contains:

- `state`: `Started`, `DriverLoaded`, `Stopped`, `Disabled`, `Problem`, or
  `Unknown`
- `problem`: optional `DeviceProblem { code, kind }`, preserving the raw
  `CM_PROB_*` value
- `flags`: raw `CM_DEVNODE_STATUS_FLAGS` for diagnostics
- helpers: `needs_restart()` and `is_healthy()`

`ProblemKind` groups the common codes: not configured, driver load failed,
driver blocked, unsigned driver, device disabled, hardware disabled, device not
present, failed start, failed install, needs restart, needs reinstall, phantom,
resource conflict, and other. Unknown codes remain `Other` with the numeric code
intact.

### Category

`DeviceCategory` is a conservative grouping derived from the Windows device
class, PCI and USB class codes, and a small set of well-known ACPI IDs. It never
uses the friendly name:

- processor, graphics, motherboard, chipset, PCI, USB
- network, Ethernet, wireless
- Bluetooth, audio, storage, storage controller, input
- unknown

`Motherboard` is assigned only for known ACPI platform IDs such as `PNP0C01` and
`PNP0C02`; the PnP class `System` alone is too broad to identify a board. Full
board identity will be supplied later from `Win32_BaseBoard` and treated as
supplemental data rather than a PnP category.

### Device record and snapshot

`DeviceRecord` carries the instance ID, ordered hardware and compatible IDs, bus
type, category, identification, class name and GUID, manufacturer, friendly
name, device description, enumerator, parent instance ID, location facts, driver
facts, status, presence flag, and a coarse `DeviceCondition`.

`DeviceCondition` is the UI-facing classification:

- `healthy` — present and started or driver-loaded
- `disabled` — present and disabled by Windows
- `problem` — present with a non-driver error
- `missing-driver` — present with an install/load/block/signature problem, or
  stopped without driver metadata
- `non-present` — installed but not connected
- `unknown` — presence or status could not be determined

`DriverInfo` carries provider, version, date (UTC), description, INF file name,
and service name. `DEVPKEY_Device_DriverInfPath` returns an INF file name, not a
full path, so the field is named `inf_name`. It is descriptive metadata only; it
does not imply that an update exists.

`HardwareSnapshot` contains the device records and `ScanDiagnostics`:

- provider name (`CfgMgr32`)
- whether only present devices were enumerated (false in Phase 10)
- device count and duration
- per-property failures
- per-device failures

## Privileges

Enumeration and property reads are designed to work for a standard user.
Windows may deny specific properties; those are degraded to diagnostics rather
than failing the scan.

Administrator rights are expected for operations that Phase 10 does not perform:

- installing, updating, or removing driver packages
- writing to the driver store or protected registry keys
- changing device state
- some WMI classes and namespaces
- driver signature or policy changes

OpenDrvr does not auto-elevate. Future installation work must be an explicit,
separately gated action with signature verification and rollback.

## Compatibility

- Target: Windows 10/11 x64.
- CfgMgr32 is available on all supported Windows versions.
- Unified device properties (`CM_Get_DevNode_PropertyW`) are available since
  Windows Vista.
- `CM_GETIDLIST_FILTER_PRESENT` is available since Windows 7; Phase 10 uses
  `CM_GETIDLIST_FILTER_NONE` and `CM_LOCATE_DEVNODE_PHANTOM` to include
  non-present devices.
- `DEVPKEY_Device_ProblemStatus` is available since Windows 8; it is not used
  yet because the provider reads the problem code from `CM_Get_DevNode_Status`.
- The `windows` crate is a Windows-only dependency. Non-Windows builds and
  non-x64 Windows builds use `UnsupportedDeviceEnumerator` and return
  `HardwareError::UnsupportedPlatform`.

### Dependency

```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.62.2", features = [
    "Win32_Devices_DeviceAndDriverInstallation",
    "Win32_Devices_Properties",
    "Win32_Foundation",
] }
```

No WMI, SetupAPI, registry, or driver-installation crate is added in Phase 10.

## Missing drivers and unknown devices

A device with no driver metadata still produces a record. Its raw hardware IDs
and instance ID remain available for matching, and its condition and status
expose the problem:

- `CM_PROB_NOT_CONFIGURED` — no configuration exists
- `CM_PROB_FAILED_INSTALL` — installation failed
- `CM_PROB_FAILED_START` — driver failed to start
- `CM_PROB_DRIVER_FAILED_LOAD` — driver could not load
- `CM_PROB_DRIVER_BLOCKED` — driver blocked by policy
- `CM_PROB_UNSIGNED_DRIVER` — unsigned driver
- `CM_PROB_DEVICE_NOT_THERE` — device is not present

Install, load, block, and signature failures map to `missing-driver`. Other
present failures map to `problem`, and disabled nodes map to `disabled`.
Non-present nodes are always `non-present`, even when the status call also
reports a problem code.

Devices with no hardware IDs, such as some root or ACPI nodes, are still
recorded with empty identifier lists. They are not dropped or fabricated.

## Tests

Run the backend checks with:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-targets
```

Coverage in Phase 10:

- PCI hardware ID parsing, including subsystem order and class codes
- USB hardware ID parsing, including interface and class triplets
- ACPI compact and `VEN`/`DEV` parsing
- HD Audio codec and subsystem parsing
- preservation of malformed and unknown identifiers
- case-insensitive prefix handling
- device instance ID validation and bus classification
- vendor lookup, aliases, and unknown-ID behavior
- identity resolution for PCI, USB, ACPI, and HD Audio
- instance-ID fallback when hardware IDs are missing
- confirmed, partial, and unresolved confidence
- conflicting manufacturer metadata and subsystem-only identity
- status mapping for started, driver-loaded, stopped, disabled, problem, and
  unknown states
- `CM_PROB_*` mapping, including unknown codes
- condition classification for healthy, disabled, missing-driver, problem,
  non-present, and unknown devices
- category classification from class names, PCI/USB class codes, and ACPI IDs
- snapshot lookup and display-name fallback
- provider injection and unsupported-platform errors
- command error sanitization for hardware failures
- multi-string decoding, GUID formatting, and FILETIME conversion
- case-insensitive instance-ID deduplication
- a real-hardware test that enumerates the local device tree, checks snapshot
  invariants, and asserts that non-present devices have the matching condition

The provider trait allows service tests to use a fake enumerator, so domain
behavior does not require a live device tree.

## Next-phase boundary

1. **Persist scans.** Use the existing `hardware_scans` and
   `hardware_scan_devices` tables. Add a migration only if the richer model
   needs fields the current schema lacks: raw compatible IDs, driver provider and
   version, problem code, category, bus type, identification confidence, and
   location facts.
2. **Match the catalog.** Match exact hardware IDs first, then compatible IDs,
   then conservative class fallbacks. Never match by friendly name. The catalog
   v2 schema stores canonical PCI, USB, ACPI, and HDAUDIO identifiers plus
   per-driver match quality and provenance, so keep parsed identity aligned with
   `HardwareId` and require an official source reference for any compatibility
   claim. Do not treat partial or unresolved identification as a confirmed match.
3. **Add WMI/CIM as supplemental data.** Use `Win32_Processor`,
   `Win32_VideoController`, `Win32_BaseBoard`, `Win32_BIOS`,
   `Win32_ComputerSystem`, `Win32_PhysicalMemory`, and
   `Win32_NetworkAdapter` for facts PnP does not expose cleanly, such as CPU
   model and cores, GPU memory, board model, BIOS version, and memory modules.
   PnP identity remains authoritative.
4. **Add read-only SetupAPI inspection.** Use driver information lists and INF
   metadata to enrich provider/version/date facts and to explain missing
   drivers. Do not install or modify anything in this step.
5. **Defer installation.** Driver download, signature verification, install,
   rollback, and elevation belong to a later phase. They must be explicit,
   auditable, and separate from scanning.

## Licensing

The Phase 10 code remains part of OpenDrvr and is licensed under **AGPL-3.0**.
No third-party source was copied. The `windows` crate is used under its
upstream MIT/Apache-2.0 license and is linked as a normal dependency.
