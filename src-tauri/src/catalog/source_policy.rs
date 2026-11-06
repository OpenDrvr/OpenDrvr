//! Official source policy for catalog imports.
//!
//! A driver record is only accepted when it points at a manufacturer-controlled
//! domain. Search engines, forums, and third-party driver aggregators are not
//! authoritative even when they mirror an official file. The domain list below
//! is curated and intentionally not exhaustive: adding a domain is a deliberate
//! policy change that requires verifying the manufacturer controls it.

use std::net::IpAddr;

use crate::catalog::model::DriverSourceKind;

/// Curated registrable domains controlled by hardware, system, and driver
/// publishers. A URL is accepted when its host is one of these domains or a
/// subdomain of one.
pub const OFFICIAL_DOMAINS: &[&str] = &[
    "acer.com",
    "ali.com.tw",
    "alpsalpine.com",
    "amd.com",
    "apple.com",
    "arm.com",
    "arubanetworks.com",
    "asmedia.com.tw",
    "aspeedtech.com",
    "asrock.com",
    "asrockrack.com",
    "asus.com",
    "audient.com",
    "behringer.com",
    "brother.com",
    "broadcom.com",
    "buffalo.jp",
    "canon.com",
    "cmedia.com.tw",
    "cisco.com",
    "cirrus.com",
    "clevo.com.tw",
    "corsair.com",
    "creative.com",
    "crucial.com",
    "dell.com",
    "displaylink.com",
    "dlink.com",
    "elan.com",
    "epson.com",
    "esi-audio.com",
    "fintek.com.tw",
    "focusrite.com",
    "frame.work",
    "fujitsu.com",
    "genesyslogic.com.tw",
    "gigabyte.com",
    "goodix.com",
    "hp.com",
    "hpe.com",
    "huawei.com",
    "hyperx.com",
    "ibm.com",
    "imaginationtech.com",
    "infineon.com",
    "intel.com",
    "ite.com.tw",
    "juniper.net",
    "kingston.com",
    "kioxia.com",
    "konica-minolta.com",
    "kyocera.com",
    "lenovo.com",
    "lexmark.com",
    "lg.com",
    "logitech.com",
    "marvell.com",
    "mediatek.com",
    "meraki.com",
    "micron.com",
    "microsoft.com",
    "motu.com",
    "msi.com",
    "native-instruments.com",
    "netgear.com",
    "nuvoton.com",
    "nvidia.com",
    "nxp.com",
    "panasonic.com",
    "parallels.com",
    "presonus.com",
    "purism.com",
    "qualcomm.com",
    "razer.com",
    "realtek.com",
    "redhat.com",
    "renesas.com",
    "ricoh.com",
    "rivernetworks.com",
    "rme-audio.de",
    "roccat.com",
    "roland.com",
    "samsung.com",
    "sandisk.com",
    "seagate.com",
    "sharp.com",
    "sis.com",
    "sk-hynix.com",
    "smsc.com",
    "sony.com",
    "st.com",
    "starlabs.systems",
    "steelseries.com",
    "steinberg.net",
    "supermicro.com",
    "synaptics.com",
    "system76.com",
    "tenda.com",
    "ti.com",
    "toshiba.com",
    "tp-link.com",
    "trendnet.com",
    "turtlebeach.com",
    "tyan.com",
    "ubiquiti.com",
    "universal-audio.com",
    "via.com.tw",
    "virtualbox.org",
    "vmware.com",
    "wacom.com",
    "westerndigital.com",
    "windows.com",
    "windowsupdate.com",
    "winbond.com",
    "xiaomi.com",
    "yamaha.com",
    "zyxel.com",
];

const MICROSOFT_DOMAINS: &[&str] = &["microsoft.com", "windows.com", "windowsupdate.com"];

/// Validates an HTTPS URL and returns its normalized host.
///
/// The host must be manufacturer-controlled, contain a public DNS name rather
/// than an IP literal, and must not embed credentials.
pub fn validate_official_url(value: &str) -> Result<String, String> {
    let value = value.trim();

    if value.is_empty() {
        return Err("URL is empty".to_string());
    }

    if value
        .chars()
        .any(|character| character.is_control() || character.is_whitespace())
    {
        return Err("URL contains whitespace or control characters".to_string());
    }

    let rest = value
        .get(..8)
        .filter(|prefix| prefix.eq_ignore_ascii_case("https://"))
        .map(|_| &value[8..])
        .ok_or_else(|| "URL must use HTTPS".to_string())?;

    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..authority_end];

    if authority.is_empty() {
        return Err("URL is missing a host".to_string());
    }

    if authority.contains('@') {
        return Err("URL must not contain credentials".to_string());
    }

    if authority.starts_with('[') {
        return Err("URL must use a DNS name, not an IP literal".to_string());
    }

    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) if !host.contains(':') => (host, Some(port)),
        _ => (authority, None),
    };

    if let Some(port) = port {
        if port.is_empty() || !port.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err("URL port is invalid".to_string());
        }

        match port.parse::<u16>() {
            Ok(0) | Err(_) => return Err("URL port is invalid".to_string()),
            Ok(_) => {}
        }
    }

    let host = host.trim_end_matches('.');
    if host.is_empty() {
        return Err("URL is missing a host".to_string());
    }

    if !host.is_ascii() {
        return Err("URL host must be an ASCII DNS name".to_string());
    }

    let host = host.to_ascii_lowercase();

    if host.parse::<IpAddr>().is_ok() {
        return Err("URL must use a DNS name, not an IP literal".to_string());
    }

    let labels: Vec<&str> = host.split('.').collect();
    if labels.len() < 2 {
        return Err("URL host must be a fully qualified DNS name".to_string());
    }

    for label in &labels {
        if label.is_empty() || label.len() > 63 {
            return Err("URL host contains an invalid DNS label".to_string());
        }

        if label.starts_with('-') || label.ends_with('-') {
            return Err("URL host contains an invalid DNS label".to_string());
        }

        if !label
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err("URL host contains an invalid DNS label".to_string());
        }
    }

    if labels.last().is_none_or(|label| label.len() < 2) {
        return Err("URL host has an invalid top-level domain".to_string());
    }

    if !is_official_host(&host) {
        return Err(format!(
            "URL host '{host}' is not in the official source registry"
        ));
    }

    Ok(host)
}

/// Returns true when the host is a curated official domain or a subdomain.
pub fn is_official_host(host: &str) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    OFFICIAL_DOMAINS
        .iter()
        .any(|domain| host_matches_domain(&host, domain))
}

/// Validates a driver source URL against the source kind. Windows Update
/// entries must stay on Microsoft-controlled domains.
pub fn validate_source_url(kind: DriverSourceKind, value: &str) -> Result<String, String> {
    let host = validate_official_url(value)?;

    if kind == DriverSourceKind::WindowsUpdate
        && !MICROSOFT_DOMAINS
            .iter()
            .any(|domain| host_matches_domain(&host, domain))
    {
        return Err("Windows Update sources must use a Microsoft-controlled domain".to_string());
    }

    Ok(host)
}

fn host_matches_domain(host: &str, domain: &str) -> bool {
    host == domain
        || (host.len() > domain.len()
            && host.ends_with(domain)
            && host.as_bytes()[host.len() - domain.len() - 1] == b'.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_official_domains_and_subdomains() {
        assert_eq!(
            validate_official_url(
                "https://www.intel.com/content/www/us/en/download-center/home.html"
            )
            .expect("intel"),
            "www.intel.com"
        );
        assert_eq!(
            validate_official_url("https://downloadmirror.intel.com/123/driver.zip")
                .expect("intel mirror"),
            "downloadmirror.intel.com"
        );
        assert_eq!(
            validate_official_url("https://catalog.update.microsoft.com/Search.aspx?q=driver")
                .expect("microsoft"),
            "catalog.update.microsoft.com"
        );
        assert_eq!(
            validate_official_url("https://ftp.hp.com/pub/caps-softpaq/cmit/").expect("hp"),
            "ftp.hp.com"
        );
        assert_eq!(
            validate_official_url("https://frame.work/support").expect("frame.work"),
            "frame.work"
        );
    }

    #[test]
    fn rejects_non_https_or_unlisted_hosts() {
        for value in [
            "http://www.intel.com/drivers",
            "https://example.com/driver.zip",
            "https://intel.com.evil.example/driver.zip",
            "https://evilintel.com/driver.zip",
            "https://www.intel.com@evil.example/driver.zip",
            "https://127.0.0.1/driver.zip",
            "https://[::1]/driver.zip",
            "https://www.intel.com:0/driver.zip",
            "https://www.intel.com:99999/driver.zip",
            "javascript:alert(1)",
            "",
        ] {
            assert!(validate_official_url(value).is_err(), "{value}");
        }
    }

    #[test]
    fn restricts_windows_update_sources_to_microsoft() {
        assert!(validate_source_url(
            DriverSourceKind::WindowsUpdate,
            "https://catalog.update.microsoft.com/Search.aspx?q=driver"
        )
        .is_ok());

        assert!(validate_source_url(
            DriverSourceKind::WindowsUpdate,
            "https://www.intel.com/drivers"
        )
        .is_err());

        assert!(validate_source_url(
            DriverSourceKind::Manufacturer,
            "https://www.intel.com/drivers"
        )
        .is_ok());
    }

    #[test]
    fn domain_matching_does_not_accept_suffix_lookalikes() {
        assert!(is_official_host("intel.com"));
        assert!(is_official_host("downloadmirror.intel.com"));
        assert!(!is_official_host("notintel.com"));
        assert!(!is_official_host("intel.com.evil.example"));
    }
}
