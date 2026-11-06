import { invoke } from "@tauri-apps/api/core";

export interface AppInfo {
  name: string;
  version: string;
  platform: string;
  architecture: string;
  profile: "debug" | "release";
  backend: string;
}

export type DatabaseComponentStatus = "ok" | "unavailable";

export interface UserDatabaseHealth {
  status: DatabaseComponentStatus;
  schemaVersion: number;
  tables: number;
  writable: boolean;
  message: string | null;
}

export type CatalogOrigin = "bundled" | "update";

export interface CatalogDatabaseHealth {
  status: DatabaseComponentStatus;
  origin: CatalogOrigin | null;
  schemaVersion: number | null;
  catalogVersion: string | null;
  generatedAt: string | null;
  lastVerifiedAt: string | null;
  tables: number;
  manufacturerCount: number;
  hardwareDeviceCount: number;
  driverSourceCount: number;
  driverCount: number;
  readOnly: boolean;
  message: string | null;
}

export interface DatabaseHealth {
  user: UserDatabaseHealth;
  catalog: CatalogDatabaseHealth;
}

export type ModuleState = "ready" | "planned" | "unavailable";

export interface ModuleStatus {
  name: string;
  state: ModuleState;
  detail: string;
}

export interface BackendStatus {
  version: string;
  platform: string;
  supportedTarget: boolean;
  modules: ModuleStatus[];
}

export type ThemePreference = "dark";
export type UpdateChannel = "stable";

export interface AppPreferences {
  theme: ThemePreference;
  updateChannel: UpdateChannel;
  downloadDirectory: string;
  checkCatalogUpdates: boolean;
  notifyDownloads: boolean;
  keepDownloadHistory: boolean;
}

/** Error payload returned by Rust commands. */
export interface CommandErrorPayload {
  code: string;
  message: string;
}

/** Normalized IPC failure with the backend's stable error code attached. */
export class IpcError extends Error {
  readonly code: string;

  constructor(payload: CommandErrorPayload) {
    super(payload.message);
    this.name = "IpcError";
    this.code = payload.code;
  }
}

function isCommandErrorPayload(value: unknown): value is CommandErrorPayload {
  if (typeof value !== "object" || value === null) {
    return false;
  }

  const candidate = value as { code?: unknown; message?: unknown };
  return typeof candidate.code === "string" && typeof candidate.message === "string";
}

/**
 * Tauri injects this global into the WebView. It is absent when the frontend
 * is opened in a regular browser via `pnpm dev`.
 */
export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function requireTauriRuntime(): void {
  if (!isTauriRuntime()) {
    throw new Error(
      "Tauri IPC is unavailable in a plain browser. Launch the desktop shell with pnpm tauri:dev.",
    );
  }
}

/**
 * Single IPC entry point. Every command goes through here so runtime checks and
 * error normalization cannot drift between callers.
 */
async function invokeCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  requireTauriRuntime();

  try {
    return await invoke<T>(command, args);
  } catch (error) {
    if (isCommandErrorPayload(error)) {
      throw new IpcError(error);
    }

    throw error;
  }
}

/** Safe application metadata used to confirm the Rust backend is reachable. */
export async function getAppInfo(): Promise<AppInfo> {
  return invokeCommand<AppInfo>("get_app_info");
}

/** Returns live health for the writable user database and the bundled catalog. */
export async function getDatabaseHealth(): Promise<DatabaseHealth> {
  return invokeCommand<DatabaseHealth>("get_database_health");
}

/** Returns the backend version, platform and per-module lifecycle state. */
export async function getBackendStatus(): Promise<BackendStatus> {
  return invokeCommand<BackendStatus>("get_backend_status");
}

/** Reads persisted application preferences. Missing values use defaults. */
export async function getPreferences(): Promise<AppPreferences> {
  return invokeCommand<AppPreferences>("get_preferences");
}

/** Validates and persists the complete preference set. */
export async function updatePreferences(preferences: AppPreferences): Promise<AppPreferences> {
  return invokeCommand<AppPreferences>("update_preferences", { preferences });
}

/** Removes saved preferences and returns the application defaults. */
export async function resetPreferences(): Promise<AppPreferences> {
  return invokeCommand<AppPreferences>("reset_preferences");
}

export type DeviceState =
  "started" | "driver-loaded" | "stopped" | "disabled" | "problem" | "unknown";

export type ProblemKind =
  | "not-configured"
  | "driver-load-failed"
  | "driver-blocked"
  | "driver-unsigned"
  | "device-disabled"
  | "hardware-disabled"
  | "device-not-present"
  | "failed-start"
  | "failed-install"
  | "needs-restart"
  | "needs-reinstall"
  | "phantom"
  | "resource-conflict"
  | "other";

export type DeviceCondition =
  "healthy" | "disabled" | "problem" | "missing-driver" | "non-present" | "unknown";

export type DeviceCategory =
  | "processor"
  | "graphics"
  | "motherboard"
  | "chipset"
  | "pci"
  | "usb"
  | "network"
  | "network-ethernet"
  | "network-wireless"
  | "bluetooth"
  | "audio"
  | "storage"
  | "storage-controller"
  | "input"
  | "unknown";

export type BusType =
  | "pci"
  | "usb"
  | "acpi"
  | "root"
  | "software-device"
  | "bluetooth"
  | "audio"
  | "storage"
  | "unknown"
  | { other: string };

export interface PciHardwareId {
  vendorId: string | null;
  deviceId: string | null;
  subsystemVendorId: string | null;
  subsystemDeviceId: string | null;
  revision: string | null;
  classCode: string | null;
}

export interface UsbHardwareId {
  vendorId: string | null;
  productId: string | null;
  revision: string | null;
  interfaceNumber: string | null;
  classCode: string | null;
  subclassCode: string | null;
  protocolCode: string | null;
}

export interface AcpiHardwareId {
  vendorId: string | null;
  deviceId: string | null;
}

export interface HdaudioHardwareId {
  functionGroup: string | null;
  vendorId: string | null;
  deviceId: string | null;
  subsystemVendorId: string | null;
  subsystemDeviceId: string | null;
  revision: string | null;
}

export type HardwareIdKind =
  | { bus: "pci"; details: PciHardwareId }
  | { bus: "usb"; details: UsbHardwareId }
  | { bus: "acpi"; details: AcpiHardwareId }
  | { bus: "hdaudio"; details: HdaudioHardwareId }
  | { bus: "unknown" };

export interface HardwareId {
  raw: string;
  kind: HardwareIdKind;
}

export interface DeviceProblem {
  code: number;
  kind: ProblemKind;
}

export interface DeviceStatus {
  state: DeviceState;
  problem: DeviceProblem | null;
  flags: number;
}

export interface DriverInfo {
  provider: string | null;
  version: string | null;
  date: string | null;
  description: string | null;
  infName: string | null;
  service: string | null;
}

export interface DeviceLocation {
  locationInfo: string | null;
  locationPaths: string[];
  busNumber: number | null;
  address: number | null;
}

export type IdentificationConfidence = "confirmed" | "partial" | "unresolved";

export type IdentifierFamily = "pci" | "usb" | "acpi" | "hdaudio" | "other";

export type VendorRole = "device" | "subsystem";

/** A vendor resolved from a known hardware identifier. */
export interface IdentifiedVendor {
  name: string;
  /** Canonical identifier value, for example `8086`, `046D`, or `INT`. */
  vendorId: string;
  family: IdentifierFamily;
  role: VendorRole;
}

/**
 * Structured identity derived from hardware identifiers.
 *
 * `manufacturer` prefers the subsystem vendor and falls back to the hardware
 * vendor. `reportedManufacturer` is the string Windows supplied and is never
 * treated as confirmed identification on its own.
 */
export interface DeviceIdentification {
  confidence: IdentificationConfidence;
  manufacturer: IdentifiedVendor | null;
  hardwareVendor: IdentifiedVendor | null;
  subsystemVendor: IdentifiedVendor | null;
  reportedManufacturer: string | null;
  reportedName: string | null;
  identifierFamilies: IdentifierFamily[];
  notes: string[];
}

export interface DeviceRecord {
  instanceId: string;
  hardwareIds: HardwareId[];
  compatibleIds: HardwareId[];
  busType: BusType;
  category: DeviceCategory;
  identification: DeviceIdentification;
  className: string | null;
  classGuid: string | null;
  manufacturer: string | null;
  friendlyName: string | null;
  deviceDescription: string | null;
  enumerator: string | null;
  parentInstanceId: string | null;
  location: DeviceLocation;
  driver: DriverInfo | null;
  status: DeviceStatus;
  isPresent: boolean;
  condition: DeviceCondition;
}

export interface PropertyFailure {
  instanceId: string;
  property: string;
  error: string;
}

export interface DeviceFailure {
  instanceId: string;
  operation: string;
  error: string;
}

export interface ScanDiagnostics {
  provider: string;
  presentOnly: boolean;
  deviceCount: number;
  propertyFailures: PropertyFailure[];
  deviceFailures: DeviceFailure[];
  durationMs: number;
}

export interface HardwareSnapshot {
  devices: DeviceRecord[];
  diagnostics: ScanDiagnostics;
}

/** Enumerates the local Windows device tree, including non-present devices. */
export async function scanHardware(): Promise<HardwareSnapshot> {
  return invokeCommand<HardwareSnapshot>("scan_hardware");
}
