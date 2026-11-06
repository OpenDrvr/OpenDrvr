# Windows hardware integration

This document describes the native hardware layer prepared in **Phase 6**. It
records the Windows API assessment, the portable data model, the provider
architecture, privilege and compatibility expectations, and the plan for the
Phase 7 scanner.

Phase 6 deliberately stops short of a user-facing scan. There is no IPC command,
no frontend hardware data, no catalog matching, and no driver download or
installation. The hardware module remains `Planned`; only the native enumeration
layer and its tests exist.

## Goals and boundaries

The future scanner must identify at least:

- CPU and GPU devices
- motherboard and chipset devices
- PCI and USB devices
- Ethernet, Wi-Fi, and Bluetooth devices
- audio devices
- storage controllers
- devices Windows cannot classify
- devices with missing, failed, or blocked drivers

Identity must come from Plug and Play data, not from friendly names. The raw
device instance ID and hardware IDs are authoritative. PCI and USB parsing is a
convenience for matching and presentation; an unrecognized or malformed string
is always preserved verbatim.

Phase 6 does **not**:

- expose a `scan_hardware` command
- render hardware data in React
- match devices against the catalog
- query WMI/CIM
- inspect or install driver packages
- download drivers
- request administrator elevation

## Windows API assessment

| API                                                   | Role                                              | Strengths                                                                                                                                                                    | Limits                                                                                                                             | Phase 6 decision                                                   |
| ----------------------------------------------------- | ------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| Configuration Manager (`cfgmgr32.dll`)                | Primary PnP device tree access                    | Machine-wide enumeration without class GUIDs; device instance IDs; status flags and problem codes; typed device properties; parent/child relationships; notification support | Lower-level than SetupAPI; driver-package/INF details are limited; some properties are unavailable to standard users               | **Used now** for enumeration, status, and device/driver properties |
| SetupAPI (`setupapi.dll`)                             | Device sets, registry properties, driver packages | Class/group enumeration; `SetupDiGetDeviceRegistryProperty`; driver info lists; INF inspection; installation APIs                                                            | Requires an `HDEVINFO` set and often a class GUID; older property model; driver-store/install operations need administrator rights | **Reserved** for future driver-package and INF inspection          |
| PnP manager / device installation APIs (`newdev.dll`) | Driver installation and updates                   | `DiInstallDriver`, `DiInstallDevice`, `DiUninstallDevice`, `UpdateDriverForPlugAndPlayDevices`                                                                               | Administrator-only, changes system state, needs rollback and signature checks                                                      | **Out of scope** until a later driver-install phase                |
| WMI/CIM (`root\cimv2`)                                | Supplemental system facts                         | `Win32_Processor`, `Win32_VideoController`, `Win32_BaseBoard`, `Win32_BIOS`, `Win32_PhysicalMemory`, `Win32_NetworkAdapter`, `Win32_PnPEntity`, `Win32_PnPSignedDriver`      | Slower; COM initialization; some classes need administrator rights; not the authoritative PnP identity                             | **Reserved** for supplemental CPU/GPU/board/BIOS/memory facts      |

### Why CfgMgr32 is primary

CfgMgr32 exposes the device tree directly and does not require the caller to
enumerate class GUIDs first. It provides:

- `CM_Get_Device_ID_List_SizeW` and `CM_Get_Device_ID_ListW` to list device
  instance IDs, with `CM_GETIDLIST_FILTER_PRESENT` for the current tree
- `CM_Locate_DevNodeW` to obtain a device node handle
- `CM_Get_DevNode_Status` for `DN_*` flags and `CM_PROB_*` problem codes
- `CM_Get_DevNode_PropertyW` for typed `DEVPKEY_Device_*` properties
- `CM_Get_Parent`, `CM_Get_Child`, and `CM_Get_Sibling` for tree traversal if a
  future view needs hierarchy beyond the stored parent ID

SetupAPI remains valuable later because it is the documented path for driver
information lists and INF/package inspection. Keeping the provider boundary
narrow means the scanner can add SetupAPI without changing the portable model.

### Property keys used in Phase 6

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
AppState
  └── HardwareService
        └── Arc<dyn DeviceEnumerator>        portable hardware::provider trait
              └── WindowsDeviceEnumerator    platform::windows::device_tree
                    └── CfgMgr32             cfgmgr32.dll through windows-rs
```

- `hardware/` owns the portable model, identifier parsing, status mapping,
  categorization, and the provider trait. It never imports Win32 or Tauri.
- `platform::device_enumerator()` constructs the platform provider. `AppState`
  injects it into `HardwareService`.
- `platform::windows::device_tree` is the only module that calls CfgMgr32. It
  returns the portable `HardwareSnapshot`.
- The dependency is inverted: the hardware domain does not depend on platform
  code. A future test or alternate provider can implement `DeviceEnumerator`
  without a Windows device tree.

`HardwareService::scan()` exists for internal use and tests, but it is not
registered as an IPC command. The module status stays `Planned` with a message
that the scan is scheduled for Phase 7.

### Enumeration flow

1. Query the required UTF-16 buffer size with
   `CM_Get_Device_ID_List_SizeW(NULL, CM_GETIDLIST_FILTER_PRESENT)`.
2. Read the multi-string device ID list with `CM_Get_Device_ID_ListW`. A
   `CR_BUFFER_SMALL` result is retried a bounded number of times because the
   device tree can change between the size query and the read.
3. For each instance ID, locate the device node with `CM_Locate_DevNodeW`.
4. Read status and problem code with `CM_Get_DevNode_Status`.
5. Read each optional property with a size query followed by a typed read.
   Missing values (`CR_NO_SUCH_VALUE`) are normal and are not diagnostics.
6. Emit one `DeviceRecord` per device, then sort by instance ID for deterministic
   output.

Only failure to obtain the device ID list is fatal. A device that cannot be
located, or whose status cannot be read, is recorded in diagnostics while the
rest of the scan continues. A property with an unexpected type or malformed
payload is recorded as a property failure and left unset.

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
- `kind` — `Pci`, `Usb`, or `Unknown`

PCI parsing recognizes `VEN`, `DEV`, `SUBSYS`, `REV`, and `CC` tokens. In
`SUBSYS_ssssdddd`, the first four hex digits are the subsystem device ID and the
last four are the subsystem vendor ID. USB parsing recognizes `VID`, `PID`,
`REV`, `MI`, `CLASS`, `SUBCLASS`, and `PROT`. Malformed fields become `None`
without changing `raw`.

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
class and PCI class code, never from the friendly name:

- processor, graphics, chipset, PCI, USB
- network, Ethernet, wireless
- Bluetooth, audio, storage controller
- unknown

`Motherboard` is reserved in the model. The PnP class `System` is too broad to
identify a board reliably, so Phase 7 will supply board identity from
`Win32_BaseBoard` and treat it as supplemental data rather than a PnP category.

### Device record and snapshot

`DeviceRecord` carries the instance ID, ordered hardware and compatible IDs, bus
type, category, class name and GUID, manufacturer, friendly name, device
description, enumerator, parent instance ID, location facts, driver facts,
status, and presence flag.

`DriverInfo` carries provider, version, date (UTC), description, INF path, and
service name. It is descriptive metadata only; it does not imply that an update
exists.

`HardwareSnapshot` contains the device records and `ScanDiagnostics`:

- provider name (`CfgMgr32`)
- whether only present devices were enumerated
- device count and duration
- per-property failures
- per-device failures

## Privileges

Enumeration and property reads are designed to work for a standard user.
Windows may deny specific properties; those are degraded to diagnostics rather
than failing the scan.

Administrator rights are expected for operations that Phase 6 does not perform:

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
- `CM_GETIDLIST_FILTER_PRESENT` is available since Windows 7.
- `DEVPKEY_Device_ProblemStatus` is available since Windows 8; it is not used
  yet because Phase 6 reads the problem code from `CM_Get_DevNode_Status`.
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

No WMI, SetupAPI, registry, or driver-installation crate is added in Phase 6.

## Missing drivers and unknown devices

A device with no driver metadata still produces a record. Its raw hardware IDs
and instance ID remain available for matching, and its status exposes the
problem:

- `CM_PROB_NOT_CONFIGURED` — no configuration exists
- `CM_PROB_FAILED_INSTALL` — installation failed
- `CM_PROB_FAILED_START` — driver failed to start
- `CM_PROB_DRIVER_FAILED_LOAD` — driver could not load
- `CM_PROB_DRIVER_BLOCKED` — driver blocked by policy
- `CM_PROB_UNSIGNED_DRIVER` — unsigned driver
- `CM_PROB_DEVICE_NOT_THERE` — device is not present

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

Coverage in Phase 6:

- PCI hardware ID parsing, including subsystem order and class codes
- USB hardware ID parsing, including interface and class triplets
- preservation of malformed and unknown identifiers
- case-insensitive prefix handling
- device instance ID validation and bus classification
- status mapping for started, driver-loaded, stopped, disabled, problem, and
  unknown states
- `CM_PROB_*` mapping, including unknown codes
- category classification from class names and PCI class codes
- snapshot lookup and display-name fallback
- provider injection and unsupported-platform errors
- multi-string decoding, GUID formatting, and FILETIME conversion
- a real-hardware test that enumerates the local present device tree and checks
  snapshot invariants

The provider trait allows service tests to use a fake enumerator, so domain
behavior does not require a live device tree.

## Phase 7 plan

1. **Expose a typed scan command.** Add `scan_hardware` returning a serializable
   DTO derived from `HardwareSnapshot`, register it in the command macro, and
   add a typed frontend wrapper. The Hardware page then renders real records,
   with explicit loading, empty, and error states.
2. **Persist scans.** Use the existing `hardware_scans` and
   `hardware_scan_devices` tables. Add a migration only if the richer model
   needs fields the current schema lacks: raw compatible IDs, driver provider and
   version, problem code, category, bus type, and location facts.
3. **Match the catalog.** Match exact hardware IDs first, then compatible IDs,
   then conservative class fallbacks. Never match by friendly name. Keep the
   catalog schema's PCI/USB fields aligned with `HardwareId`.
4. **Add WMI/CIM as supplemental data.** Use `Win32_Processor`,
   `Win32_VideoController`, `Win32_BaseBoard`, `Win32_BIOS`,
   `Win32_ComputerSystem`, `Win32_PhysicalMemory`, and
   `Win32_NetworkAdapter` for facts PnP does not expose cleanly, such as CPU
   model and cores, GPU memory, board model, BIOS version, and memory modules.
   PnP identity remains authoritative.
5. **Add read-only SetupAPI inspection.** Use driver information lists and INF
   metadata to enrich provider/version/date facts and to explain missing
   drivers. Do not install or modify anything in this step.
6. **Add diagnostics and refresh.** Surface scan duration, degraded properties,
   and failures in a developer-facing diagnostics view. Add a manual refresh and
   a cached snapshot.
7. **Defer installation.** Driver download, signature verification, install,
   rollback, and elevation belong to a later phase. They must be explicit,
   auditable, and separate from scanning.

## Licensing

The Phase 6 code remains part of OpenDrvr and is licensed under **AGPL-3.0**.
No third-party source was copied. The `windows` crate is used under its
upstream MIT/Apache-2.0 license and is linked as a normal dependency.
