use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::loxcc::LoxCcError;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ConnectionStatus {
    #[default]
    Disconnected,
    Connecting {
        host: String,
        message: String,
    },
    Connected {
        host: String,
        server_info: String,
    },
    Error {
        host: String,
        error: String,
    },
}

impl ConnectionStatus {
    pub fn is_connected(&self) -> bool {
        matches!(self, Self::Connected { .. })
    }

    pub fn is_connecting(&self) -> bool {
        matches!(self, Self::Connecting { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupSnapshot {
    pub filename: String,
    pub config_version: Option<u32>,
    pub timestamp_raw: Option<String>,
    pub formatted_date: String,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionProfile {
    pub host: String,
    pub username: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(default)]
    pub remember_password: bool,
}

impl Default for ConnectionProfile {
    fn default() -> Self {
        Self {
            host: String::new(),
            username: "admin".to_string(),
            password: None,
            remember_password: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MiniserverCredentials {
    pub host: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Error)]
pub enum MiniserverError {
    #[error("Miniserver host/IP is required")]
    EmptyHost,
    #[error("Authentication failed (HTTP 401/403 or invalid FTP credentials)")]
    AuthenticationFailed,
    #[error("No project backup archives found in /prog/ on Miniserver")]
    NoBackupsFound,
    #[error("Transport error connecting to Miniserver ({0})")]
    Transport(String),
    #[error("Failed to unpack LoxCC project archive: {0}")]
    Unpack(#[from] LoxCcError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Profile JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Miniserver has a newer project version ({}): pull and merge server changes before uploading", .0.diff.reason)]
    RemoteVersionNewer(Box<crate::merge::ServerVersionConflict>),
}

pub fn parse_prog_listing(listing: &str) -> Vec<BackupSnapshot> {
    let mut snapshots = Vec::new();

    for raw_line in listing.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }
        let filename = tokens.last().unwrap().trim_matches('/').to_string();
        let is_prog_archive = filename == "sps0.LoxCC"
            || filename == "sps_new.zip"
            || (filename.starts_with("sps_")
                && (filename.ends_with(".zip") || filename.ends_with(".LoxCC")));
        if !is_prog_archive {
            continue;
        }

        let size_bytes = if tokens.len() >= 9 {
            tokens[4].parse::<u64>().ok()
        } else {
            None
        };

        let mut config_version = None;
        let mut timestamp_raw = None;
        let mut formatted_date = if filename == "sps_new.zip" {
            "Latest staging archive (sps_new.zip)".to_string()
        } else if filename == "sps0.LoxCC" {
            "Active configuration (sps0.LoxCC)".to_string()
        } else {
            "Snapshot archive".to_string()
        };

        if let Some(stem) = filename
            .strip_prefix("sps_")
            .and_then(|s| s.strip_suffix(".zip").or_else(|| s.strip_suffix(".LoxCC")))
        {
            if let Some((ver_str, ts_str)) = stem.split_once('_') {
                if let Ok(ver) = ver_str.parse::<u32>() {
                    if ts_str.len() == 14 && ts_str.chars().all(|c| c.is_ascii_digit()) {
                        config_version = Some(ver);
                        timestamp_raw = Some(ts_str.to_string());
                        formatted_date = format!(
                            "{}-{}-{} {}:{}:{}",
                            &ts_str[0..4],
                            &ts_str[4..6],
                            &ts_str[6..8],
                            &ts_str[8..10],
                            &ts_str[10..12],
                            &ts_str[12..14]
                        );
                    }
                }
            }
        }

        snapshots.push(BackupSnapshot {
            filename,
            config_version,
            timestamp_raw,
            formatted_date,
            size_bytes,
        });
    }

    snapshots.sort_by(|a, b| match (&a.timestamp_raw, &b.timestamp_raw) {
        (Some(ts_a), Some(ts_b)) => ts_b
            .cmp(ts_a)
            .then_with(|| b.config_version.cmp(&a.config_version)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => {
            let rank = |name: &str| match name {
                "sps_new.zip" => 0,
                "sps0.LoxCC" => 1,
                _ => 2,
            };
            rank(&a.filename)
                .cmp(&rank(&b.filename))
                .then_with(|| a.filename.cmp(&b.filename))
        }
    });

    snapshots
}

impl ConnectionProfile {
    pub fn default_config_path() -> Option<PathBuf> {
        let home = std::env::var_os("HOME")?;
        Some(
            PathBuf::from(home)
                .join(".config")
                .join("loxone-portable")
                .join("connection.json"),
        )
    }

    pub fn load_from_path(path: &Path) -> Result<Self, MiniserverError> {
        let raw = std::fs::read_to_string(path)?;
        let mut profile: Self = serde_json::from_str(&raw)?;
        if !profile.remember_password {
            profile.password = None;
        }
        Ok(profile)
    }

    pub fn save_to_path(&self, path: &Path) -> Result<(), MiniserverError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut sanitized = self.clone();
        if !sanitized.remember_password {
            sanitized.password = None;
        }
        let json = serde_json::to_string_pretty(&sanitized)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn default_backup_dir() -> Option<PathBuf> {
        let home = std::env::var_os("HOME")?;
        Some(
            PathBuf::from(home)
                .join(".config")
                .join("loxone-portable")
                .join("backups"),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadProjectResult {
    pub backup_path: Option<PathBuf>,
    pub uploaded_filename: String,
    pub restarted_sps: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoxoneKeyChallenge {
    pub key_hex: String,
    pub salt_hex: String,
    pub hash_alg: String,
}

pub fn parse_getkey2_response(body: &str) -> Option<LoxoneKeyChallenge> {
    let val: serde_json::Value = serde_json::from_str(body.trim()).ok()?;
    let ll = val.get("LL")?;
    let v = ll.get("value")?;
    let key_hex = v.get("key")?.as_str()?.trim().to_string();
    let salt_hex = v.get("salt")?.as_str()?.trim().to_string();
    let hash_alg = v
        .get("hashAlg")
        .and_then(|a| a.as_str())
        .unwrap_or("SHA1")
        .trim()
        .to_ascii_uppercase();
    if key_hex.is_empty() || salt_hex.is_empty() {
        return None;
    }
    Some(LoxoneKeyChallenge {
        key_hex,
        salt_hex,
        hash_alg,
    })
}

fn sha1_digest(data: &[u8]) -> [u8; 20] {
    let mut h0: u32 = 0x67452301;
    let mut h1: u32 = 0xEFCDAB89;
    let mut h2: u32 = 0x98BADCFE;
    let mut h3: u32 = 0x10325476;
    let mut h4: u32 = 0xC3D2E1F0;

    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h0, h1, h2, h3, h4);
        for (i, &wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A827999),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                _ => (b ^ c ^ d, 0xCA62C1D6),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }

    let mut out = [0u8; 20];
    out[0..4].copy_from_slice(&h0.to_be_bytes());
    out[4..8].copy_from_slice(&h1.to_be_bytes());
    out[8..12].copy_from_slice(&h2.to_be_bytes());
    out[12..16].copy_from_slice(&h3.to_be_bytes());
    out[16..20].copy_from_slice(&h4.to_be_bytes());
    out
}

fn hmac_sha1(key: &[u8], message: &[u8]) -> [u8; 20] {
    let mut k_block = [0u8; 64];
    if key.len() > 64 {
        let hashed = sha1_digest(key);
        k_block[..20].copy_from_slice(&hashed);
    } else {
        k_block[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5cu8; 64];
    for i in 0..64 {
        ipad[i] ^= k_block[i];
        opad[i] ^= k_block[i];
    }
    let mut inner = Vec::with_capacity(64 + message.len());
    inner.extend_from_slice(&ipad);
    inner.extend_from_slice(message);
    let inner_hash = sha1_digest(&inner);

    let mut outer = Vec::with_capacity(64 + 20);
    outer.extend_from_slice(&opad);
    outer.extend_from_slice(&inner_hash);
    sha1_digest(&outer)
}

fn decode_hex_bytes(hex: &str) -> Vec<u8> {
    let bytes = hex.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 2);
    let mut i = 0;
    while i + 1 < bytes.len() {
        if let Ok(s) = std::str::from_utf8(&bytes[i..i + 2]) {
            if let Ok(b) = u8::from_str_radix(s, 16) {
                out.push(b);
            }
        }
        i += 2;
    }
    out
}

fn encode_hex(bytes: &[u8], uppercase: bool) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        if uppercase {
            s.push_str(&format!("{:02X}", b));
        } else {
            s.push_str(&format!("{:02x}", b));
        }
    }
    s
}

/// Computes the Loxone `/jdev/sys/getkey2` challenge response hash (`HMAC(key, "user:UPPER_HASH(pass:salt)")`).
pub fn compute_loxone_auth_hash(
    username: &str,
    password: &str,
    challenge: &LoxoneKeyChallenge,
) -> String {
    let pw_salt = format!("{}:{}", password, challenge.salt_hex);
    let pw_hash = encode_hex(&sha1_digest(pw_salt.as_bytes()), true);
    let user_payload = format!("{}:{}", username, pw_hash);
    let key_bytes = decode_hex_bytes(&challenge.key_hex);
    encode_hex(&hmac_sha1(&key_bytes, user_payload.as_bytes()), false)
}

pub struct MiniserverClient;

impl MiniserverClient {
    fn normalize_base_url(host: &str) -> Result<String, MiniserverError> {
        let trimmed = host.trim().trim_end_matches('/');
        if trimmed.is_empty() {
            return Err(MiniserverError::EmptyHost);
        }
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            Ok(trimmed.to_string())
        } else {
            Ok(format!("http://{}", trimmed))
        }
    }

    fn basic_auth_header(creds: &MiniserverCredentials) -> String {
        use base64::engine::general_purpose::STANDARD;
        use base64::Engine;
        let token = STANDARD.encode(format!("{}:{}", creds.username, creds.password));
        format!("Basic {}", token)
    }

    fn http_get_bytes(url: &str, creds: &MiniserverCredentials) -> Result<Vec<u8>, MiniserverError> {
        let agent = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(8))
            .build();
        match agent
            .get(url)
            .set("Authorization", &Self::basic_auth_header(creds))
            .call()
        {
            Ok(response) => {
                let mut bytes = Vec::new();
                std::io::Read::read_to_end(&mut response.into_reader(), &mut bytes)?;
                Ok(bytes)
            }
            Err(ureq::Error::Status(401 | 403, _)) => Err(MiniserverError::AuthenticationFailed),
            Err(ureq::Error::Status(code, _)) => {
                Err(MiniserverError::Transport(format!("HTTP status {}", code)))
            }
            Err(ureq::Error::Transport(err)) => Err(MiniserverError::Transport(err.to_string())),
        }
    }

    fn ftp_host_and_port(host: &str) -> String {
        let stripped = host
            .trim()
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .trim_start_matches("ftp://")
            .split('/')
            .next()
            .unwrap_or("")
            .trim();
        if stripped.contains(':') {
            stripped.to_string()
        } else {
            format!("{}:21", stripped)
        }
    }

    fn ftp_list_snapshots(
        creds: &MiniserverCredentials,
    ) -> Result<Vec<BackupSnapshot>, MiniserverError> {
        let addr = Self::ftp_host_and_port(&creds.host);
        let mut ftp = suppaftp::FtpStream::connect(&addr)
            .map_err(|e| MiniserverError::Transport(format!("FTP connect {}: {}", addr, e)))?;
        ftp.login(&creds.username, &creds.password)
            .map_err(|_| MiniserverError::AuthenticationFailed)?;
        let lines = ftp
            .list(Some("/prog"))
            .map_err(|e| MiniserverError::Transport(format!("FTP list /prog: {}", e)))?;
        let _ = ftp.quit();
        Ok(parse_prog_listing(&lines.join("\n")))
    }

    fn ftp_download_file(
        creds: &MiniserverCredentials,
        filename: &str,
    ) -> Result<Vec<u8>, MiniserverError> {
        let addr = Self::ftp_host_and_port(&creds.host);
        let mut ftp = suppaftp::FtpStream::connect(&addr)
            .map_err(|e| MiniserverError::Transport(format!("FTP connect {}: {}", addr, e)))?;
        ftp.login(&creds.username, &creds.password)
            .map_err(|_| MiniserverError::AuthenticationFailed)?;
        let _ = ftp.transfer_type(suppaftp::types::FileType::Binary);
        let remote_path = format!("/prog/{}", filename);
        let cursor = ftp
            .retr_as_buffer(&remote_path)
            .map_err(|e| MiniserverError::Transport(format!("FTP retr {}: {}", remote_path, e)))?;
        let _ = ftp.quit();
        Ok(cursor.into_inner())
    }

    pub fn check_connection(creds: &MiniserverCredentials) -> Result<String, MiniserverError> {
        let snapshots = Self::list_snapshots(creds)?;
        if let Some(latest) = snapshots.first() {
            Ok(format!(
                "{} backup(s) available · latest: {}",
                snapshots.len(),
                latest.filename
            ))
        } else {
            Ok("Connected (0 backups in /prog)".to_string())
        }
    }

    pub fn list_snapshots(
        creds: &MiniserverCredentials,
    ) -> Result<Vec<BackupSnapshot>, MiniserverError> {
        let base_url = Self::normalize_base_url(&creds.host)?;
        let list_url = format!("{}/dev/fslist/prog", base_url);
        match Self::http_get_bytes(&list_url, creds) {
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes);
                Ok(parse_prog_listing(&text))
            }
            Err(MiniserverError::AuthenticationFailed) => {
                Err(MiniserverError::AuthenticationFailed)
            }
            Err(http_err) => Self::ftp_list_snapshots(creds).or(Err(http_err)),
        }
    }

    pub fn download_snapshot_xml(
        creds: &MiniserverCredentials,
        filename: Option<&str>,
    ) -> Result<(String, String), MiniserverError> {
        let base_url = Self::normalize_base_url(&creds.host)?;
        let target_filename = match filename {
            Some(name) if !name.trim().is_empty() => name.trim().to_string(),
            _ => {
                let snapshots = Self::list_snapshots(creds)?;
                snapshots
                    .into_iter()
                    .next()
                    .ok_or(MiniserverError::NoBackupsFound)?
                    .filename
            }
        };

        let download_url = format!("{}/dev/fsget/prog/{}", base_url, target_filename);
        let payload = match Self::http_get_bytes(&download_url, creds) {
            Ok(bytes) => bytes,
            Err(MiniserverError::AuthenticationFailed) => {
                return Err(MiniserverError::AuthenticationFailed);
            }
            Err(http_err) => {
                Self::ftp_download_file(creds, &target_filename).or(Err(http_err))?
            }
        };

        let xml = crate::loxcc::extract_loxone_xml(&payload)?;
        Ok((target_filename, xml))
    }

    fn http_get_bytes_fast(
        url: &str,
        creds: &MiniserverCredentials,
    ) -> Result<Vec<u8>, MiniserverError> {
        let agent = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_millis(1800))
            .build();
        match agent
            .get(url)
            .set("Authorization", &Self::basic_auth_header(creds))
            .call()
        {
            Ok(response) => {
                let mut bytes = Vec::new();
                std::io::Read::read_to_end(&mut response.into_reader(), &mut bytes)?;
                Ok(bytes)
            }
            Err(ureq::Error::Status(401 | 403, _)) => Err(MiniserverError::AuthenticationFailed),
            Err(ureq::Error::Status(code, _)) => {
                Err(MiniserverError::Transport(format!("HTTP status {}", code)))
            }
            Err(ureq::Error::Transport(err)) => Err(MiniserverError::Transport(err.to_string())),
        }
    }

    pub fn fetch_io_state(
        creds: &MiniserverCredentials,
        control_id: &str,
    ) -> Result<LiveControlSnapshot, MiniserverError> {
        let base_url = Self::normalize_base_url(&creds.host)?;
        let clean_id = control_id.trim();
        let all_url = format!("{}/dev/sps/io/{}/all", base_url, clean_id);
        let bytes = Self::http_get_bytes_fast(&all_url, creds)?;
        let text = String::from_utf8_lossy(&bytes);
        if let Some(mut snap) = parse_ll_io_response(&text) {
            if snap.control_id.is_empty() {
                snap.control_id = clean_id.to_string();
            }
            if snap.code == 200 || snap.primary_value.is_some() || !snap.outputs.is_empty() {
                return Ok(snap);
            }
        }
        let state_url = format!("{}/dev/sps/io/{}/state", base_url, clean_id);
        let state_bytes = Self::http_get_bytes_fast(&state_url, creds)?;
        let state_text = String::from_utf8_lossy(&state_bytes);
        let mut snap = parse_ll_io_response(&state_text).unwrap_or_else(|| LiveControlSnapshot {
            control_id: clean_id.to_string(),
            primary_value: None,
            code: 404,
            outputs: std::collections::BTreeMap::new(),
        });
        if snap.control_id.is_empty() {
            snap.control_id = clean_id.to_string();
        }
        Ok(snap)
    }

    pub fn fetch_batch_io_states(
        creds: &MiniserverCredentials,
        control_ids: &[String],
    ) -> Vec<(String, Result<LiveControlSnapshot, MiniserverError>)> {
        control_ids
            .iter()
            .map(|id| (id.clone(), Self::fetch_io_state(creds, id)))
            .collect()
    }

    pub fn send_io_command(
        creds: &MiniserverCredentials,
        control_id: &str,
        command: &str,
    ) -> Result<LiveControlSnapshot, MiniserverError> {
        let base_url = Self::normalize_base_url(&creds.host)?;
        let clean_id = control_id.trim();
        let clean_cmd = command.trim();
        let url = format!("{}/dev/sps/io/{}/{}", base_url, clean_id, clean_cmd);
        let bytes = Self::http_get_bytes_fast(&url, creds)?;
        let text = String::from_utf8_lossy(&bytes);
        let mut snap = parse_ll_io_response(&text).unwrap_or_else(|| LiveControlSnapshot {
            control_id: clean_id.to_string(),
            primary_value: Some(clean_cmd.to_string()),
            code: 200,
            outputs: std::collections::BTreeMap::new(),
        });
        if snap.control_id.is_empty() {
            snap.control_id = clean_id.to_string();
        }
        Ok(snap)
    }

    /// Triggers physical LED / acoustic identification on a Loxone Link extension, Tree device,
    /// or Air device connected to the Miniserver.
    pub fn identify_device(
        creds: &MiniserverCredentials,
        target: &crate::periphery_provisioning::DeviceIdentifyTarget,
    ) -> Result<String, MiniserverError> {
        let base_url = Self::normalize_base_url(&creds.host)?;
        let mut last_err = None;
        let mut succeeded_endpoint = None;

        let mut candidate_paths = Vec::new();
        if !target.serial.trim().is_empty() {
            let s = target.serial.trim();
            candidate_paths.push(format!("/jdev/sys/identify/{}", s));
            candidate_paths.push(format!("/jdev/bus/identify/{}", s));
            candidate_paths.push(format!("/dev/sys/identify/{}", s));
        }
        if !target.uuid.trim().is_empty() {
            let u = target.uuid.trim();
            candidate_paths.push(format!("/dev/sps/io/{}/identify", u));
            candidate_paths.push(format!("/dev/sps/io/{}/Pulse", u));
        }

        for path in &candidate_paths {
            let url = format!("{}{}", base_url, path);
            match Self::http_get_bytes_fast(&url, creds) {
                Ok(_) => {
                    succeeded_endpoint = Some(path.clone());
                    break;
                }
                Err(MiniserverError::AuthenticationFailed) => {
                    return Err(MiniserverError::AuthenticationFailed);
                }
                Err(e) => {
                    last_err = Some(e);
                }
            }
        }

        if let Some(fb_uuid) = &target.feedback_io_uuid {
            let _ = Self::send_io_command(creds, fb_uuid, "Pulse");
        }

        if let Some(ep) = succeeded_endpoint {
            Ok(format!(
                "Identify signal sent to {} ({}) via {}",
                target.title,
                if target.serial.is_empty() {
                    target.bus_kind.label()
                } else {
                    target.serial.as_str()
                },
                ep
            ))
        } else if let Some(err) = last_err {
            Err(err)
        } else {
            Ok(format!("Identify signal queued for {}", target.title))
        }
    }

    fn http_post_bytes(
        url: &str,
        creds: &MiniserverCredentials,
        payload: &[u8],
    ) -> Result<Vec<u8>, MiniserverError> {
        let agent = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(12))
            .build();
        match agent
            .post(url)
            .set("Authorization", &Self::basic_auth_header(creds))
            .set("Content-Type", "application/octet-stream")
            .send_bytes(payload)
        {
            Ok(response) => {
                let mut bytes = Vec::new();
                std::io::Read::read_to_end(&mut response.into_reader(), &mut bytes)?;
                Ok(bytes)
            }
            Err(ureq::Error::Status(401 | 403, _)) => Err(MiniserverError::AuthenticationFailed),
            Err(ureq::Error::Status(code, _)) => {
                Err(MiniserverError::Transport(format!("HTTP status {}", code)))
            }
            Err(ureq::Error::Transport(err)) => Err(MiniserverError::Transport(err.to_string())),
        }
    }

    fn ftp_upload_file(
        creds: &MiniserverCredentials,
        filename: &str,
        payload: &[u8],
    ) -> Result<(), MiniserverError> {
        let addr = Self::ftp_host_and_port(&creds.host);
        let mut ftp = suppaftp::FtpStream::connect(&addr)
            .map_err(|e| MiniserverError::Transport(format!("FTP connect {}: {}", addr, e)))?;
        ftp.login(&creds.username, &creds.password)
            .map_err(|_| MiniserverError::AuthenticationFailed)?;
        let _ = ftp.transfer_type(suppaftp::types::FileType::Binary);
        let remote_path = format!("/prog/{}", filename);
        let mut cursor = std::io::Cursor::new(payload);
        ftp.put_file(&remote_path, &mut cursor)
            .map_err(|e| MiniserverError::Transport(format!("FTP put {}: {}", remote_path, e)))?;
        let _ = ftp.quit();
        Ok(())
    }

    /// Performs the Loxone `/jdev/sys/getkey2/{user}` + `/jdev/sys/getjwt/{hash}/{user}/4/...`
    /// challenge-response handshake and returns the issued token if supported by the Miniserver.
    pub fn exchange_loxone_jwt(creds: &MiniserverCredentials) -> Result<String, MiniserverError> {
        let base_url = Self::normalize_base_url(&creds.host)?;
        let key_url = format!("{}/jdev/sys/getkey2/{}", base_url, creds.username.trim());
        let key_bytes = Self::http_get_bytes_fast(&key_url, creds)?;
        let key_text = String::from_utf8_lossy(&key_bytes);
        let challenge = parse_getkey2_response(&key_text).ok_or_else(|| {
            MiniserverError::Transport("Invalid /jdev/sys/getkey2 challenge payload".to_string())
        })?;
        let auth_hash = compute_loxone_auth_hash(&creds.username, &creds.password, &challenge);
        let jwt_url = format!(
            "{}/jdev/sys/getjwt/{}/{}/4/098802e1-02b4-603c-ffffeee000d80cfd/loxone-portable",
            base_url,
            auth_hash,
            creds.username.trim()
        );
        let jwt_bytes = Self::http_get_bytes_fast(&jwt_url, creds)?;
        let jwt_text = String::from_utf8_lossy(&jwt_bytes);
        let val: serde_json::Value = serde_json::from_str(jwt_text.trim())?;
        if let Some(token) = val
            .get("LL")
            .and_then(|ll| ll.get("value"))
            .and_then(|v| v.get("token"))
            .and_then(|t| t.as_str())
        {
            Ok(token.to_string())
        } else {
            Err(MiniserverError::AuthenticationFailed)
        }
    }

    /// Safely deploys a `.Loxone` XML project to a Miniserver:
    /// 1. Downloads the currently running configuration first and writes a timestamped safety backup.
    /// 2. Compresses the new XML into `sps0.LoxCC` and packages `sps_new.zip`.
    /// 3. Uploads `sps_new.zip` (and `sps0.LoxCC`) to `/prog/` via HTTP `/dev/fsput/` or FTP `/prog/`.
    /// 4. Optionally triggers `/dev/sps/restart` so the Miniserver loads the new configuration.
    pub fn upload_project_xml(
        creds: &MiniserverCredentials,
        xml: &str,
        backup_dir: Option<&Path>,
        restart_sps: bool,
    ) -> Result<UploadProjectResult, MiniserverError> {
        Self::upload_project_xml_with_safeguard(
            creds,
            xml,
            None,
            None,
            false,
            backup_dir,
            restart_sps,
        )
    }

    /// Uploads a project XML configuration to the Miniserver with a newer-version safeguard:
    /// 1. Downloads the currently running configuration first and checks if the server holds a
    ///    newer version or unmerged changes (unless `force_overwrite` is true). If so, blocks the
    ///    upload with `MiniserverError::RemoteVersionNewer` so the user can pull & merge first.
    /// 2. Writes a timestamped safety backup of the running configuration.
    /// 3. Compresses the new XML into `sps0.LoxCC` and packages `sps_new.zip`.
    /// 4. Uploads `sps_new.zip` (and `sps0.LoxCC`) to `/prog/`.
    /// 5. Optionally triggers `/dev/sps/restart` so the Miniserver loads the new configuration.
    pub fn upload_project_xml_with_safeguard(
        creds: &MiniserverCredentials,
        xml: &str,
        base_xml: Option<&str>,
        last_synced_remote_xml: Option<&str>,
        force_overwrite: bool,
        backup_dir: Option<&Path>,
        restart_sps: bool,
    ) -> Result<UploadProjectResult, MiniserverError> {
        let base_url = Self::normalize_base_url(&creds.host)?;

        // Step 1: Download currently active configuration on Miniserver for safeguard check & safety backup
        let mut backup_path = None;
        if let Ok((orig_filename, running_xml)) = Self::download_snapshot_xml(creds, None) {
            if !force_overwrite {
                if let (Ok(local_proj), Ok(remote_proj)) = (
                    crate::parser::parse_loxone_xml(xml),
                    crate::parser::parse_loxone_xml(&running_xml),
                ) {
                    let base_proj =
                        base_xml.and_then(|b| crate::parser::parse_loxone_xml(b).ok());
                    if let Some(diff) = crate::merge::check_server_newer_version(
                        &local_proj,
                        &remote_proj,
                        &orig_filename,
                        base_proj.as_ref(),
                        last_synced_remote_xml,
                    ) {
                        return Err(MiniserverError::RemoteVersionNewer(Box::new(
                            crate::merge::ServerVersionConflict {
                                remote_filename: orig_filename,
                                remote_xml: running_xml,
                                diff,
                            },
                        )));
                    }
                }
            }

            let resolved_dir = backup_dir
                .map(Path::to_path_buf)
                .or_else(ConnectionProfile::default_backup_dir);
            if let Some(dir) = resolved_dir {
                let _ = std::fs::create_dir_all(&dir);
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let stem = orig_filename
                    .trim_end_matches(".zip")
                    .trim_end_matches(".LoxCC");
                let file_path = dir.join(format!("pre_upload_{}_{}.Loxone", ts, stem));
                if std::fs::write(&file_path, running_xml.as_bytes()).is_ok() {
                    backup_path = Some(file_path);
                }
            }
        }

        // Step 2: Pack .LoxCC and sps_new.zip
        let zip_bytes = crate::loxcc::pack_loxcc_zip(xml)?;
        let loxcc_bytes = crate::loxcc::compress_loxcc_literals(xml.as_bytes());

        // Step 3: Upload to /prog/sps_new.zip and /prog/sps0.LoxCC
        let put_zip_url = format!("{}/dev/fsput/prog/sps_new.zip", base_url);
        let put_loxcc_url = format!("{}/dev/fsput/prog/sps0.LoxCC", base_url);
        match Self::http_post_bytes(&put_zip_url, creds, &zip_bytes) {
            Ok(_) => {
                let _ = Self::http_post_bytes(&put_loxcc_url, creds, &loxcc_bytes);
            }
            Err(MiniserverError::AuthenticationFailed) => {
                return Err(MiniserverError::AuthenticationFailed);
            }
            Err(http_err) => {
                Self::ftp_upload_file(creds, "sps_new.zip", &zip_bytes).or(Err(http_err))?;
                let _ = Self::ftp_upload_file(creds, "sps0.LoxCC", &loxcc_bytes);
            }
        }

        // Step 4: Optionally restart SPS program
        let mut restarted_sps = false;
        if restart_sps {
            let restart_url = format!("{}/dev/sps/restart", base_url);
            let _ = Self::http_get_bytes_fast(&restart_url, creds);
            restarted_sps = true;
        }

        Ok(UploadProjectResult {
            backup_path,
            uploaded_filename: "sps_new.zip".to_string(),
            restarted_sps,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LiveSignal {
    pub raw: String,
    pub formatted: String,
    pub numeric: Option<f64>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LiveControlSnapshot {
    pub control_id: String,
    pub primary_value: Option<String>,
    pub code: u32,
    pub outputs: std::collections::BTreeMap<String, String>,
}

pub fn parse_ll_io_response(xml: &str) -> Option<LiveControlSnapshot> {
    let trimmed = xml.trim().trim_start_matches('\u{FEFF}');
    let nodes = crate::parser::parse_raw_dom(trimmed).ok()?;
    let root = nodes.iter().find(|n| n.tag.eq_ignore_ascii_case("LL"))?;

    let raw_control = root.attr("control").unwrap_or("");
    let control_id = strip_sps_io_control_path(raw_control);
    let primary_value = root
        .attr("value")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let code = root
        .attr("Code")
        .or_else(|| root.attr("code"))
        .and_then(|c| c.parse::<u32>().ok())
        .unwrap_or(200);

    let mut outputs = std::collections::BTreeMap::new();
    for child in &root.children {
        let key = child
            .attr("name")
            .or_else(|| child.attr("id"))
            .or_else(|| child.attr("control").and_then(|c| c.rsplit('/').next()));
        let val = child.attr("value").or(child.text.as_deref());
        if let (Some(k), Some(v)) = (key, val) {
            let k_clean = k.trim();
            let v_clean = v.trim();
            if !k_clean.is_empty() {
                outputs.insert(k_clean.to_string(), v_clean.to_string());
            }
        }
    }

    Some(LiveControlSnapshot {
        control_id,
        primary_value,
        code,
        outputs,
    })
}

fn strip_sps_io_control_path(raw: &str) -> String {
    let s = raw
        .trim()
        .trim_start_matches('/')
        .trim_start_matches("dev/sps/io/");
    let s = s
        .strip_suffix("/all")
        .or_else(|| s.strip_suffix("/state"))
        .or_else(|| s.strip_suffix("/on"))
        .or_else(|| s.strip_suffix("/off"))
        .or_else(|| s.strip_suffix("/impuls"))
        .or_else(|| s.strip_suffix("/pulse"))
        .unwrap_or(s);
    s.split('/').next().unwrap_or(s).to_string()
}

pub fn format_live_signal(
    raw: &str,
    hint_title: &str,
    hint_pin_key: &str,
    hint_block_type: &str,
) -> LiveSignal {
    let trimmed = raw.trim();
    let title_lower = hint_title.to_lowercase();
    let key_upper = hint_pin_key.to_uppercase();

    // 1. Touch Pure / T5 multi-button state ("0/0/0/0")
    if hint_pin_key == "T5"
        || hint_pin_key == "Sel1"
        || hint_pin_key == "Gesture"
        || hint_title == "T5"
        || trimmed.contains("/0/")
    {
        if trimmed.contains('/') {
            let is_active = trimmed.split('/').any(|p| p.trim() != "0" && !p.trim().is_empty());
            return LiveSignal {
                raw: trimmed.to_string(),
                formatted: trimmed.to_string(),
                numeric: Some(if is_active { 1.0 } else { 0.0 }),
                is_active,
            };
        }
        let is_on = matches!(trimmed, "1" | "on" | "On" | "true" | "Wł.");
        return LiveSignal {
            raw: trimmed.to_string(),
            formatted: if is_on {
                "1/0/0/0".to_string()
            } else {
                "0/0/0/0".to_string()
            },
            numeric: Some(if is_on { 1.0 } else { 0.0 }),
            is_active: is_on,
        };
    }

    // 2. Smart Actuator / LightController2 OutputAPI / SMA ("WW(0%)" / "WW(1%)")
    let is_sma = hint_pin_key == "OutputAPI"
        || hint_pin_key == "SMA"
        || trimmed.starts_with("WW(")
        || trimmed.starts_with("RGB(")
        || (hint_block_type == "OutputRef" && title_lower.contains("oświetlenie"));
    if is_sma {
        if trimmed.starts_with("WW(") || trimmed.starts_with("RGB(") {
            let is_active = !trimmed.contains("(0%)");
            return LiveSignal {
                raw: trimmed.to_string(),
                formatted: trimmed.to_string(),
                numeric: Some(if is_active { 1.0 } else { 0.0 }),
                is_active,
            };
        }
        let num = parse_numeric_prefix(trimmed).unwrap_or(0.0);
        let pct = if (trimmed == "1" || trimmed.eq_ignore_ascii_case("on") || trimmed == "Wł.")
            && !trimmed.ends_with('%')
        {
            100
        } else {
            num.round() as i64
        };
        return LiveSignal {
            raw: trimmed.to_string(),
            formatted: format!("WW({}%)", pct),
            numeric: Some(pct as f64),
            is_active: pct > 0,
        };
    }

    // 3. Temperature ("22.9°")
    let is_temp = hint_pin_key == "Temp"
        || hint_pin_key == "ϑc"
        || title_lower.contains("temperatura")
        || trimmed.ends_with('°')
        || trimmed.ends_with("°C");
    if is_temp {
        let num = parse_numeric_prefix(trimmed).unwrap_or(0.0);
        let formatted = if trimmed.ends_with('°') {
            trimmed.to_string()
        } else if let Some(stripped) = trimmed.strip_suffix("°C") {
            format!("{}°", stripped.trim())
        } else if (num.fract()).abs() < 1e-4 {
            format!("{:.0}°", num)
        } else {
            format!("{:.1}°", num)
        };
        return LiveSignal {
            raw: trimmed.to_string(),
            formatted,
            numeric: Some(num),
            is_active: num.abs() > 1e-6,
        };
    }

    // 4. Humidity ("45%")
    let is_humid = hint_pin_key == "InHumid"
        || hint_pin_key == "Humid"
        || title_lower.contains("wilgotno")
        || title_lower.contains("humid")
        || (trimmed.ends_with('%') && !trimmed.contains('('));
    if is_humid {
        let num = parse_numeric_prefix(trimmed).unwrap_or(0.0);
        let formatted = if trimmed.ends_with('%') {
            trimmed.to_string()
        } else {
            format!("{:.0}%", num)
        };
        return LiveSignal {
            raw: trimmed.to_string(),
            formatted,
            numeric: Some(num),
            is_active: num.abs() > 1e-6,
        };
    }

    // 5. Brightness / Lux ("0Lx")
    let is_lux = hint_pin_key == "Brightness"
        || hint_pin_key == "Br"
        || title_lower.contains("jasno")
        || title_lower.contains("bright")
        || trimmed.ends_with("Lx")
        || trimmed.ends_with("lx");
    if is_lux {
        let num = parse_numeric_prefix(trimmed).unwrap_or(0.0);
        let formatted = if trimmed.ends_with("Lx") {
            trimmed.to_string()
        } else if let Some(s) = trimmed.strip_suffix("lx") {
            format!("{}Lx", s.trim())
        } else {
            format!("{:.0}Lx", num)
        };
        return LiveSignal {
            raw: trimmed.to_string(),
            formatted,
            numeric: Some(num),
            is_active: num.abs() > 1e-6,
        };
    }

    // 6. Explicit Polish / English digital strings
    if matches!(trimmed, "Wł." | "on" | "On" | "ON" | "true" | "True") {
        return LiveSignal {
            raw: trimmed.to_string(),
            formatted: "Wł.".to_string(),
            numeric: Some(1.0),
            is_active: true,
        };
    }
    if matches!(trimmed, "Wył." | "off" | "Off" | "OFF" | "false" | "False") {
        return LiveSignal {
            raw: trimmed.to_string(),
            formatted: "Wył.".to_string(),
            numeric: Some(0.0),
            is_active: false,
        };
    }

    // 7. Numeric / Analog pins (I1, I2, HC1, HC2, V1, V2, H1, H2, C1, C2, AQ, AI, Zawór)
    let is_light_controller_aq = matches!(
        hint_block_type,
        "LightController2" | "LightControllerV2" | "LightController"
    ) && key_upper.starts_with("AQ");
    let is_analog_pin = !is_light_controller_aq
        && (matches!(
            key_upper.as_str(),
            "I1" | "I2" | "I3" | "I4" | "HC1" | "HC2" | "V1" | "V2" | "H1" | "H2" | "C1" | "C2" | "AQ" | "AI" | "VQ" | "VAL" | "POS"
        ) || key_upper.starts_with("AI")
            || key_upper.starts_with("AQ")
            || title_lower.contains("zawór"));

    if let Some(num) = parse_numeric_prefix(trimmed) {
        if is_analog_pin || (num != 0.0 && num != 1.0) {
            let formatted = if (num.fract()).abs() < 1e-5 {
                format!("{}", num as i64)
            } else {
                format!("{}", (num * 100.0).round() / 100.0)
            };
            return LiveSignal {
                raw: trimmed.to_string(),
                formatted,
                numeric: Some(num),
                is_active: num.abs() > 1e-6,
            };
        }
        // Digital 0 / 1
        let is_on = num.abs() > 1e-6;
        return LiveSignal {
            raw: trimmed.to_string(),
            formatted: if is_on {
                "Wł.".to_string()
            } else {
                "Wył.".to_string()
            },
            numeric: Some(num),
            is_active: is_on,
        };
    }

    if is_analog_pin {
        return LiveSignal {
            raw: "0".to_string(),
            formatted: "0".to_string(),
            numeric: Some(0.0),
            is_active: false,
        };
    }

    LiveSignal {
        raw: if trimmed.is_empty() {
            "0".to_string()
        } else {
            trimmed.to_string()
        },
        formatted: if trimmed.is_empty() {
            "Wył.".to_string()
        } else {
            trimmed.to_string()
        },
        numeric: None,
        is_active: !trimmed.is_empty() && trimmed != "0",
    }
}

fn parse_numeric_prefix(s: &str) -> Option<f64> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    let prefix: String = trimmed
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-' || *c == '+')
        .collect();
    if prefix.is_empty() || prefix == "-" || prefix == "+" || prefix == "." {
        None
    } else {
        prefix.parse::<f64>().ok()
    }
}

/// Maps Loxone XML `<Co K="...">` pin keys to the compact Loxone Config schematic symbol
/// shown in `live.png` (e.g., `Temp` -> `ϑc`, `InHumid` -> `H`, `Brightness` -> `Br`,
/// `Presence` -> `P`, `Move` -> `M`, `Sel1`/`Gesture` -> `T5`, `OutputAPI` -> `SMA`,
/// `Stop` -> `Dwc`, `Reset` -> `R`, `H1` -> `V1`, `H2` -> `V2`, `Dem` -> `H`, `Cool` -> `C`).
pub fn loxone_short_pin_label<'a>(block_type: &str, pin_key: &'a str) -> &'a str {
    match (block_type, pin_key) {
        (_, "Temp") => "ϑc",
        (_, "InHumid") | (_, "Humid") => "H",
        (_, "Brightness") => "Br",
        (_, "Presence") => "P",
        (_, "Move") => "M",
        (_, "Sel1") | (_, "Gesture") => "T5",
        (_, "OutputAPI") => "SMA",
        (_, "Stop") => "Dwc",
        (_, "Reset") => "R",
        (_, "VolUp") => "V+",
        (_, "VolDown") => "V-",
        (_, "InputTrigger") => "Tr",
        ("HeatIRoomController2", "Dem") => "H",
        ("HeatIRoomController2", "Cool") => "C",
        ("HeatIRoomController2", "AQh") | ("HeatIRoomController2", "H1") => "V1",
        ("HeatIRoomController2", "AQc") | ("HeatIRoomController2", "H2") => "V2",
        _ => pin_key,
    }
}

/// Computes the compact pin-key capsule badge shown on the connection side of an
/// `InputRef`, `OutputRef`, `OutputRefLM`, or `Memory` reference pill in `live.png`
/// (`T5`, `Br`, `P`, `M`, `ϑc`, `H`, `SMA`, `VQ`, `Q14`, `O`, `->`).
pub fn ref_bar_capsule_badge(
    block: &crate::model::PageBlock,
    page: &crate::model::Page,
    project: &crate::model::LoxoneProject,
) -> String {
    let title_lower = block.title.to_lowercase();
    if block.block_type == "Memory" {
        return "->".to_string();
    }

    if block.is_input_ref_bar() {
        if block.title == "T5" {
            return "T5".to_string();
        }
        if title_lower.contains("temperatura") {
            return "ϑc".to_string();
        }
        if title_lower.contains("wilgotno") {
            return "H".to_string();
        }
        if title_lower.contains("jasno") {
            return "Br".to_string();
        }
        // Check which downstream pin on the current page is wired to this InputRef
        // (e.g., Presence -> "P" vs Move -> "M" for "Obecność")
        for other in &page.blocks {
            if other.uuid == block.uuid {
                continue;
            }
            for dst_pin in &other.pins {
                for link in &dst_pin.incoming_links {
                    if block.pins.iter().any(|p| p.uuid == link.source_pin_uuid) {
                        let short = loxone_short_pin_label(&other.block_type, &dst_pin.key);
                        if !short.is_empty() && short.chars().count() <= 4 {
                            return short.to_string();
                        }
                    }
                }
            }
        }
        if title_lower.contains("obecno") {
            return "P".to_string();
        }
        return "I".to_string();
    }

    if block.is_output_ref_bar() {
        if block.block_type == "OutputRefLM" {
            return "->".to_string();
        }
        for pin in &block.pins {
            for link in &pin.incoming_links {
                if let Some(src_meta) = project.xref.pins_by_uuid.get(&link.source_pin_uuid) {
                    if src_meta.pin_key == "OutputAPI"
                        || src_meta.pin_key == "SMA"
                        || (src_meta.owner_type == "LightController2"
                            && title_lower.contains("oświetlenie"))
                    {
                        return "SMA".to_string();
                    }
                    if src_meta.pin_key == "AQh"
                        || src_meta.pin_key == "H1"
                        || src_meta.pin_key == "H2"
                        || src_meta.pin_key == "V1"
                        || src_meta.pin_key == "V2"
                        || title_lower.contains("zawór")
                        || title_lower.contains("ogrzewanie")
                    {
                        return "VQ".to_string();
                    }
                    if src_meta.pin_key == "Dem" {
                        return "->".to_string();
                    }
                    if src_meta.pin_key == "O" {
                        return "O".to_string();
                    }
                    if title_lower.contains("podcięcie") {
                        return "Q14".to_string();
                    }
                }
            }
        }
        if title_lower.contains("oświetlenie") {
            return "SMA".to_string();
        }
        if title_lower.contains("zawór") || title_lower.contains("ogrzewanie") {
            return "VQ".to_string();
        }
        if title_lower.contains("głośnik") {
            return "O".to_string();
        }
        if title_lower.contains("kominek") {
            return "->".to_string();
        }
        return "Q".to_string();
    }

    "IO".to_string()
}

/// Classifies the interactive on-canvas control widget shown inside a reference bar
/// ("small block") when LiveView is active (matching `on.png`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefBarLiveControlKind {
    /// Pill toggle switch (`(●)` / `(○)`) for binary/presence/switch states (`P`, `I`, `Q`, `SMA`, `Sps`, etc.).
    ToggleSwitch,
    /// Momentary pushbutton (`(●)` circle) for pulse/trigger/movement inputs (`M`, `Tr`, `R`, `T`, `I+`).
    Pushbutton,
    /// 5-button touch pad icon (`❖`) for Loxone Touch `T5` inputs.
    Touch5Pad,
    /// Numeric / float value badge (`#`) for analog sensors and outputs (`Br`, `ϑ`, `ϑc`, `H`, `VQ`, `AQ`, `AI`).
    NumericValue,
    /// No inline interactive control on the pill.
    None,
}

pub fn classify_ref_bar_live_control(
    block: &crate::model::PageBlock,
    badge: &str,
    _project: &crate::model::LoxoneProject,
) -> RefBarLiveControlKind {
    if !block.is_ref_bar() {
        return RefBarLiveControlKind::None;
    }

    let badge_trim = badge.trim();
    if badge_trim.eq_ignore_ascii_case("T5") || block.title.trim().eq_ignore_ascii_case("T5") {
        return RefBarLiveControlKind::Touch5Pad;
    }

    let badge_upper = badge_trim.to_uppercase();
    if matches!(
        badge_upper.as_str(),
        "BR" | "H" | "VQ" | "AQ" | "AI" | "V1" | "V2"
    ) || badge_trim.contains('ϑ')
    {
        return RefBarLiveControlKind::NumericValue;
    }

    let title_lower = block.title.to_lowercase();
    if title_lower.contains("temperatura")
        || title_lower.contains("wilgotno")
        || title_lower.contains("jasno")
        || title_lower.contains("zawór")
        || title_lower.contains("analog")
    {
        return RefBarLiveControlKind::NumericValue;
    }

    if matches!(
        badge_upper.as_str(),
        "M" | "TR" | "R" | "T" | "I+" | "I-" | "PLUS" | "MINUS" | "RESET"
    ) || title_lower.contains("przycisk")
        || title_lower.contains("impuls")
    {
        return RefBarLiveControlKind::Pushbutton;
    }

    if badge_trim == "->" {
        return RefBarLiveControlKind::None;
    }

    RefBarLiveControlKind::ToggleSwitch
}

#[derive(Debug, Clone, Default)]
pub struct LiveValueStore {
    pub controls: std::collections::HashMap<String, LiveControlSnapshot>,
    pub pin_overrides: std::collections::HashMap<(String, String), String>,
    pub poll_count: u64,
    pub last_error: Option<String>,
}

impl LiveValueStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Seeds realistic default / initial values for all project controls and sensors
    /// so LiveView displays immediate, accurate values even before the first HTTP poll
    /// or when inspecting a project offline (matching `live.png`).
    pub fn seed_project_defaults(&mut self, project: &crate::model::LoxoneProject) {
        for page in &project.pages {
            for block in &page.blocks {
                let mut snap = self
                    .controls
                    .remove(&block.uuid)
                    .unwrap_or_else(|| LiveControlSnapshot {
                        control_id: block.uuid.clone(),
                        primary_value: None,
                        code: 200,
                        outputs: std::collections::BTreeMap::new(),
                    });

                let title_lower = block.title.to_lowercase();
                if snap.primary_value.is_none() {
                    if block.is_input_ref_bar() {
                        if block.title == "T5" {
                            snap.primary_value = Some("0/0/0/0".to_string());
                        } else if title_lower.contains("temperatura") {
                            snap.primary_value = Some("22.9°".to_string());
                        } else if title_lower.contains("wilgotno") {
                            snap.primary_value = Some("45%".to_string());
                        } else if title_lower.contains("jasno") {
                            snap.primary_value = Some("0Lx".to_string());
                        } else {
                            snap.primary_value = Some("0".to_string());
                        }
                    } else if block.block_type == "HeatIRoomController2" {
                        snap.outputs
                            .entry("AQh".to_string())
                            .or_insert_with(|| "0.8".to_string());
                        snap.outputs
                            .entry("H1".to_string())
                            .or_insert_with(|| "0.8".to_string());
                        snap.outputs
                            .entry("Dem".to_string())
                            .or_insert_with(|| "0".to_string());
                    } else if block.block_type == "LightController2" {
                        snap.outputs
                            .entry("OutputAPI".to_string())
                            .or_insert_with(|| "WW(0%)".to_string());
                        snap.outputs
                            .entry("AQ1".to_string())
                            .or_insert_with(|| "WW(0%)".to_string());
                        snap.outputs
                            .entry("AQ3".to_string())
                            .or_insert_with(|| "0".to_string());
                    }
                }

                for pin in &block.pins {
                    if let Some(def) = &pin.default_value {
                        if !def.is_empty() {
                            snap.outputs
                                .entry(pin.key.clone())
                                .or_insert_with(|| def.clone());
                        }
                    }
                }

                if let Some(ref_uuid) = &block.ref_uuid {
                    if let Some(primary) = &snap.primary_value {
                        let ref_snap = self.controls.entry(ref_uuid.clone()).or_insert_with(|| {
                            LiveControlSnapshot {
                                control_id: ref_uuid.clone(),
                                primary_value: Some(primary.clone()),
                                code: 200,
                                outputs: std::collections::BTreeMap::new(),
                            }
                        });
                        if ref_snap.primary_value.is_none() {
                            ref_snap.primary_value = Some(primary.clone());
                        }
                    }
                }

                self.controls.insert(block.uuid.clone(), snap);
            }
        }
    }

    pub fn apply_polled_snapshot(&mut self, id: String, snapshot: LiveControlSnapshot) {
        self.controls.insert(id, snapshot);
    }

    pub fn set_pin_override(&mut self, block_uuid: &str, pin_key: &str, raw_value: &str) {
        self.pin_overrides.insert(
            (block_uuid.to_string(), pin_key.to_string()),
            raw_value.to_string(),
        );
        let snap = self
            .controls
            .entry(block_uuid.to_string())
            .or_insert_with(|| LiveControlSnapshot {
                control_id: block_uuid.to_string(),
                primary_value: None,
                code: 200,
                outputs: std::collections::BTreeMap::new(),
            });
        snap.primary_value = Some(raw_value.to_string());
        snap.outputs
            .insert(pin_key.to_string(), raw_value.to_string());
    }

    pub fn set_control_value(&mut self, control_uuid: &str, raw_value: &str) {
        self.pin_overrides.insert(
            (control_uuid.to_string(), String::new()),
            raw_value.to_string(),
        );
        let snap = self
            .controls
            .entry(control_uuid.to_string())
            .or_insert_with(|| LiveControlSnapshot {
                control_id: control_uuid.to_string(),
                primary_value: None,
                code: 200,
                outputs: std::collections::BTreeMap::new(),
            });
        snap.primary_value = Some(raw_value.to_string());
    }

    /// Applies an interactive user command (toggle switch, pushbutton, T5 pad, or numeric input)
    /// and synchronizes connected upstream/downstream pins on the schematic so wires, function
    /// block outputs, and reference pills stay consistent. Returns the resolved Miniserver IO target UUID.
    pub fn apply_interactive_command(
        &mut self,
        project: Option<&crate::model::LoxoneProject>,
        control_uuid: &str,
        pin_key: Option<&str>,
        raw_value: &str,
    ) -> String {
        if let Some(pk) = pin_key {
            self.set_pin_override(control_uuid, pk, raw_value);
            return control_uuid.to_string();
        }

        self.set_control_value(control_uuid, raw_value);
        for k in ["O", "Q", "AQ", "AI", "I"] {
            self.set_pin_override(control_uuid, k, raw_value);
        }

        let mut target_io_id = control_uuid.to_string();
        let Some(proj) = project else {
            return target_io_id;
        };

        let mut matched_info: Option<(usize, String, Option<String>, String)> = None;
        for (page_idx, page) in proj.pages.iter().enumerate() {
            if let Some(blk) = page.blocks.iter().find(|b| b.uuid == control_uuid) {
                let badge = ref_bar_capsule_badge(blk, page, proj);
                if let Some(ref_u) = &blk.ref_uuid {
                    target_io_id = ref_u.clone();
                }
                matched_info = Some((
                    page_idx,
                    blk.title.clone(),
                    blk.ref_uuid.clone(),
                    badge.clone(),
                ));

                if !blk.is_input_ref_bar() {
                    // Direct toggle on an OutputRef / Memory: synchronize the connected upstream
                    // source pin(s) so both ends of the wire and the upstream pin dot match!
                    let mut upstream_pins: Vec<(String, String)> = Vec::new();
                    for pin in &blk.pins {
                        for link in &pin.incoming_links {
                            if link.is_hidden() {
                                continue;
                            }
                            if let Some(src_blk) = page.blocks.iter().find(|b| {
                                b.pins.iter().any(|p| p.uuid == link.source_pin_uuid)
                            }) {
                                if let Some(src_pin) =
                                    src_blk.pins.iter().find(|p| p.uuid == link.source_pin_uuid)
                                {
                                    upstream_pins
                                        .push((src_blk.uuid.clone(), src_pin.key.clone()));
                                }
                            } else if let Some(meta) =
                                proj.xref.pins_by_uuid.get(&link.source_pin_uuid)
                            {
                                upstream_pins
                                    .push((meta.owner_uuid.clone(), meta.pin_key.clone()));
                            }
                        }
                    }
                    let upstream_val = if badge == "SMA" {
                        match raw_value {
                            "1" | "On" | "on" => "WW(100%)",
                            "0" | "Off" | "off" => "WW(0%)",
                            other => other,
                        }
                    } else {
                        raw_value
                    };
                    for (src_blk_u, src_pin_k) in upstream_pins {
                        self.set_pin_override(&src_blk_u, &src_pin_k, upstream_val);
                    }
                } else {
                    // Toggling an InputRef: clear stale direct overrides on downstream function
                    // block output pins and OutputRef bars on this page so the new input propagates!
                    let input_pin_uuids: Vec<String> =
                        blk.pins.iter().map(|p| p.uuid.clone()).collect();
                    let mut affected_block_uuids: Vec<String> = Vec::new();
                    for other in &page.blocks {
                        if other.pins.iter().any(|dp| {
                            dp.incoming_links
                                .iter()
                                .any(|l| input_pin_uuids.contains(&l.source_pin_uuid))
                        }) {
                            affected_block_uuids.push(other.uuid.clone());
                        }
                    }
                    let mut affected_out_pin_uuids: Vec<String> = Vec::new();
                    for other in &page.blocks {
                        if affected_block_uuids.contains(&other.uuid) {
                            for p in &other.pins {
                                affected_out_pin_uuids.push(p.uuid.clone());
                                self.pin_overrides
                                    .remove(&(other.uuid.clone(), p.key.clone()));
                            }
                            self.pin_overrides
                                .remove(&(other.uuid.clone(), String::new()));
                        }
                    }
                    for other in &page.blocks {
                        if other.is_output_ref_bar()
                            && other.pins.iter().any(|dp| {
                                dp.incoming_links.iter().any(|l| {
                                    input_pin_uuids.contains(&l.source_pin_uuid)
                                        || affected_out_pin_uuids.contains(&l.source_pin_uuid)
                                })
                            })
                        {
                            for k in ["", "O", "Q", "AQ", "AI", "I"] {
                                self.pin_overrides
                                    .remove(&(other.uuid.clone(), k.to_string()));
                            }
                            if let Some(snap) = self.controls.get_mut(&other.uuid) {
                                snap.primary_value = None;
                                snap.outputs.clear();
                            }
                        }
                    }
                }
                break;
            }
        }

        if let Some((_, src_title, Some(ref_u), badge)) = matched_info {
            let mut shares_multi_channel_device = false;
            for page in &proj.pages {
                for blk in &page.blocks {
                    if blk.ref_uuid.as_deref() == Some(ref_u.as_str()) {
                        let other_badge = ref_bar_capsule_badge(blk, page, proj);
                        if other_badge != badge || blk.title != src_title {
                            shares_multi_channel_device = true;
                        } else if blk.uuid != control_uuid {
                            self.set_control_value(&blk.uuid, raw_value);
                        }
                    }
                }
            }
            if !shares_multi_channel_device {
                self.set_control_value(&ref_u, raw_value);
                self.set_pin_override(&ref_u, "O", raw_value);
                self.set_pin_override(&ref_u, "Q", raw_value);
            }
        } else {
            for page in &proj.pages {
                for blk in &page.blocks {
                    if blk.ref_uuid.as_deref() == Some(control_uuid) {
                        self.set_control_value(&blk.uuid, raw_value);
                    }
                }
            }
        }

        target_io_id
    }

    /// Collects the control UUIDs that should be polled on the Miniserver for the given page.
    pub fn collect_poll_ids_for_page(
        page: &crate::model::Page,
        _project: &crate::model::LoxoneProject,
    ) -> Vec<String> {
        let mut ids = Vec::new();
        for block in &page.blocks {
            if block.is_note() {
                continue;
            }
            if let Some(ref_uuid) = &block.ref_uuid {
                if !ids.contains(ref_uuid) {
                    ids.push(ref_uuid.clone());
                }
            }
            if !ids.contains(&block.uuid) {
                ids.push(block.uuid.clone());
            }
        }
        ids
    }

    /// Resolves the live signal for a reference bar (`InputRef`, `OutputRef`, `OutputRefLM`, `Memory`).
    pub fn resolve_ref_bar_signal(
        &self,
        block: &crate::model::PageBlock,
        page: &crate::model::Page,
        project: &crate::model::LoxoneProject,
    ) -> LiveSignal {
        let badge = ref_bar_capsule_badge(block, page, project);

        // 1. Check direct user override on this specific block first
        for pin_k in &["", "O", "AQ", "Q", "AI", "I"] {
            if let Some(ov) = self
                .pin_overrides
                .get(&(block.uuid.clone(), (*pin_k).to_string()))
            {
                return format_live_signal(ov, &block.title, &badge, &block.block_type);
            }
        }

        // 2. If this is an OutputRef or Memory with an incoming wire from a block on the page,
        // propagate the upstream source pin's signal!
        if !block.is_input_ref_bar() {
            for pin in &block.pins {
                for link in &pin.incoming_links {
                    if let Some(sig) =
                        self.resolve_pin_by_uuid(&link.source_pin_uuid, page, project, 0)
                    {
                        return format_live_signal(&sig.raw, &block.title, &badge, &block.block_type);
                    }
                }
            }
        }

        // 3. Check control snapshot on block.uuid first (channel-specific), then ref_uuid
        if let Some(snap) = self.controls.get(&block.uuid) {
            if let Some(val) = snap
                .primary_value
                .as_deref()
                .or_else(|| snap.outputs.values().next().map(|s| s.as_str()))
            {
                return format_live_signal(val, &block.title, &badge, &block.block_type);
            }
        }

        if let Some(ref_uuid) = &block.ref_uuid {
            if let Some(snap) = self.controls.get(ref_uuid) {
                if let Some(val) = snap
                    .primary_value
                    .as_deref()
                    .or_else(|| snap.outputs.values().next().map(|s| s.as_str()))
                {
                    return format_live_signal(val, &block.title, &badge, &block.block_type);
                }
            }
        }

        format_live_signal("0", &block.title, &badge, &block.block_type)
    }

    /// Resolves the live signal on a specific `(block, pin)` pair, following incoming wire links
    /// from upstream source pins (`InputRef`s, other function blocks, or `TreeNode`s).
    pub fn resolve_pin_signal(
        &self,
        block: &crate::model::PageBlock,
        pin: &crate::model::ConnectorPin,
        page: &crate::model::Page,
        project: &crate::model::LoxoneProject,
    ) -> LiveSignal {
        self.resolve_pin_signal_depth(block, pin, page, project, 0)
    }

    fn resolve_pin_signal_depth(
        &self,
        block: &crate::model::PageBlock,
        pin: &crate::model::ConnectorPin,
        page: &crate::model::Page,
        project: &crate::model::LoxoneProject,
        depth: usize,
    ) -> LiveSignal {
        if let Some(ov) = self
            .pin_overrides
            .get(&(block.uuid.clone(), pin.key.clone()))
        {
            return format_live_signal(ov, &block.title, &pin.key, &block.block_type);
        }

        if block.is_ref_bar() {
            return self.resolve_ref_bar_signal(block, page, project);
        }

        // If this pin has incoming wire links, propagate from the first active (or first available) source pin
        if depth < 4 && !pin.incoming_links.is_empty() {
            let mut first_resolved: Option<LiveSignal> = None;
            for link in &pin.incoming_links {
                if link.is_hidden() {
                    continue;
                }
                if let Some(src_sig) =
                    self.resolve_pin_by_uuid(&link.source_pin_uuid, page, project, depth + 1)
                {
                    let formatted = format_live_signal(
                        &src_sig.raw,
                        &block.title,
                        &pin.key,
                        &block.block_type,
                    );
                    if formatted.is_active {
                        return formatted;
                    }
                    if first_resolved.is_none() {
                        first_resolved = Some(formatted);
                    }
                }
            }
            if let Some(sig) = first_resolved {
                return sig;
            }
        }

        // For output pins on lighting, switch, pushbutton, and logic blocks, evaluate
        // live output state from connected upstream input pins when any input is active or overridden.
        if depth < 4 && pin.incoming_links.is_empty() {
            if let Some(eval_sig) =
                self.evaluate_function_block_output(block, pin, page, project, depth)
            {
                return eval_sig;
            }
        }

        if let Some(snap) = self.controls.get(&block.uuid) {
            if let Some(v) = snap.outputs.get(&pin.key) {
                return format_live_signal(v, &block.title, &pin.key, &block.block_type);
            }
            if let Some(v) = &snap.primary_value {
                if matches!(pin.key.as_str(), "Q" | "AQ" | "O" | "OutputAPI") {
                    return format_live_signal(v, &block.title, &pin.key, &block.block_type);
                }
            }
        }

        if let Some(def) = &pin.default_value {
            if !def.is_empty() {
                return format_live_signal(def, &block.title, &pin.key, &block.block_type);
            }
        }

        format_live_signal("0", &block.title, &pin.key, &block.block_type)
    }

    fn evaluate_function_block_output(
        &self,
        block: &crate::model::PageBlock,
        pin: &crate::model::ConnectorPin,
        page: &crate::model::Page,
        project: &crate::model::LoxoneProject,
        depth: usize,
    ) -> Option<LiveSignal> {
        let is_lighting_or_switch_block = matches!(
            block.block_type.as_str(),
            "LightController2"
                | "LightControllerV2"
                | "LightController"
                | "PushButton"
                | "Pushbutton"
                | "Switch"
                | "StairwellLight"
                | "OffDelay"
                | "Monoflop"
                | "PresenceLight"
                | "Or"
                | "And"
                | "BurglarAlarm"
                | "TextState"
                | "Radio"
        );
        if !is_lighting_or_switch_block {
            return None;
        }

        let is_output_pin = pin.key == "OutputAPI"
            || pin.key == "Q"
            || pin.key == "Qon"
            || pin.key == "Qoff"
            || pin.key == "O"
            || pin.key.starts_with("AQ")
            || pin.key.starts_with("Q")
            || pin.key.starts_with("Level");
        if !is_output_pin {
            return None;
        }

        let mut any_trigger_active = false;
        let mut all_logic_inputs_active = true;
        let mut has_logic_inputs = false;
        let mut any_reset_active = false;
        let mut any_input_overridden = false;

        for in_pin in &block.pins {
            if in_pin.incoming_links.is_empty() {
                continue;
            }
            let key = in_pin.key.as_str();
            let is_reset_pin = matches!(
                key,
                "Reset" | "R" | "Off" | "Dis" | "InputDisable" | "Inactive" | "Confirm"
            );
            let is_trigger_pin = matches!(
                key,
                "Presence"
                    | "Move"
                    | "Gesture"
                    | "Plus"
                    | "Minus"
                    | "On"
                    | "InputTrigger"
                    | "InputTriggerUp"
                    | "InputTriggerDown"
                    | "Tr"
                    | "S"
                    | "I"
                    | "ActiveDelay"
                    | "ActiveDelayP"
                    | "Alarm"
                    | "Urgent"
            ) || key.starts_with("Sel")
                || key.starts_with("I");

            if !is_reset_pin && !is_trigger_pin {
                continue;
            }

            for link in &in_pin.incoming_links {
                if link.is_hidden() {
                    continue;
                }
                if let Some(src_blk) = page
                    .blocks
                    .iter()
                    .find(|b| b.pins.iter().any(|p| p.uuid == link.source_pin_uuid))
                {
                    if self
                        .pin_overrides
                        .keys()
                        .any(|(u, _)| u == &src_blk.uuid)
                    {
                        any_input_overridden = true;
                    }
                }
                if let Some(sig) =
                    self.resolve_pin_by_uuid(&link.source_pin_uuid, page, project, depth + 1)
                {
                    if is_reset_pin && sig.is_active {
                        any_reset_active = true;
                    } else if is_trigger_pin {
                        has_logic_inputs = true;
                        if sig.is_active {
                            any_trigger_active = true;
                        } else {
                            all_logic_inputs_active = false;
                        }
                    }
                }
            }
        }

        if !any_trigger_active && !any_reset_active && !any_input_overridden {
            return None;
        }

        let block_active = if any_reset_active {
            false
        } else if block.block_type == "And" {
            has_logic_inputs && all_logic_inputs_active
        } else {
            any_trigger_active
        };

        let out_active = if pin.key == "Qoff" {
            !block_active
        } else {
            block_active
        };

        let uses_ww_format = self
            .controls
            .get(&block.uuid)
            .and_then(|s| s.outputs.get(&pin.key))
            .is_some_and(|v| v.starts_with("WW("));

        let raw = if uses_ww_format || pin.key == "OutputAPI" {
            if out_active {
                "WW(100%)"
            } else {
                "WW(0%)"
            }
        } else if out_active {
            "1"
        } else {
            "0"
        };

        Some(format_live_signal(
            raw,
            &block.title,
            &pin.key,
            &block.block_type,
        ))
    }

    pub fn resolve_pin_by_uuid(
        &self,
        pin_uuid: &str,
        page: &crate::model::Page,
        project: &crate::model::LoxoneProject,
        depth: usize,
    ) -> Option<LiveSignal> {
        if depth > 5 {
            return None;
        }
        // 1. Check blocks on the current page first
        for blk in &page.blocks {
            if let Some(pin) = blk.pins.iter().find(|p| p.uuid == pin_uuid) {
                return Some(self.resolve_pin_signal_depth(blk, pin, page, project, depth));
            }
        }
        // 2. Check across all pages via xref
        if let Some(pin_meta) = project.xref.pins_by_uuid.get(pin_uuid) {
            for p in &project.pages {
                if let Some(blk) = p.blocks.iter().find(|b| b.uuid == pin_meta.owner_uuid) {
                    if let Some(pin) = blk.pins.iter().find(|cp| cp.uuid == pin_uuid) {
                        return Some(self.resolve_pin_signal_depth(blk, pin, p, project, depth));
                    }
                }
            }
            // Or TreeNode in periphery/system tree
            if let Some(snap) = self.controls.get(&pin_meta.owner_uuid) {
                if let Some(v) = snap
                    .outputs
                    .get(&pin_meta.pin_key)
                    .or(snap.primary_value.as_ref())
                {
                    return Some(format_live_signal(
                        v,
                        &pin_meta.owner_title,
                        &pin_meta.pin_key,
                        &pin_meta.owner_type,
                    ));
                }
            }
            return Some(format_live_signal(
                "0",
                &pin_meta.owner_title,
                &pin_meta.pin_key,
                &pin_meta.owner_type,
            ));
        }
        None
    }

    pub fn resolve_tree_node_signal(&self, node: &crate::model::TreeNode) -> Option<LiveSignal> {
        if let Some(snap) = self.controls.get(&node.uuid) {
            if let Some(val) = snap
                .primary_value
                .as_deref()
                .or_else(|| snap.outputs.values().next().map(|s| s.as_str()))
            {
                return Some(format_live_signal(val, &node.title, "", &node.control_type));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Read, Write};
    use std::net::TcpListener;
    use zip::write::SimpleFileOptions;

    #[test]
    fn test_parse_prog_listing_sorts_newest_timestamp_first() {
        let sample = "\
-rw-rw-rw-   1 0        0          142000 Feb 14 10:15 sps_192_20260214101530.zip\r\n\
-rw-rw-rw-   1 0        0          158262 Mar  8 18:22 sps_194_20260308182256.zip\r\n\
-rw-rw-rw-   1 0        0          150111 Mar  1 09:05 sps_193_20260301090512.zip\r\n\
-rw-rw-rw-   1 0        0          158262 Mar  8 18:22 sps_new.zip\r\n\
-rw-rw-rw-   1 0        0           89120 Mar  8 18:22 sps0.LoxCC\r\n\
-rw-rw-rw-   1 0        0            1024 Jan  1 00:00 Default.ico\r\n";

        let snapshots = parse_prog_listing(sample);
        assert_eq!(snapshots.len(), 5);
        assert_eq!(snapshots[0].filename, "sps_194_20260308182256.zip");
        assert_eq!(snapshots[0].config_version, Some(194));
        assert_eq!(snapshots[0].formatted_date, "2026-03-08 18:22:56");
        assert_eq!(snapshots[0].size_bytes, Some(158_262));

        assert_eq!(snapshots[1].filename, "sps_193_20260301090512.zip");
        assert_eq!(snapshots[2].filename, "sps_192_20260214101530.zip");
        assert_eq!(snapshots[3].filename, "sps_new.zip");
        assert_eq!(snapshots[4].filename, "sps0.LoxCC");
    }

    #[test]
    fn test_connection_profile_strips_password_when_remember_is_false() {
        let temp_dir = std::env::temp_dir().join(format!(
            "loxone_portable_profile_test_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp_dir);
        let profile_path = temp_dir.join("connection.json");

        let profile_no_remember = ConnectionProfile {
            host: "192.168.1.100".to_string(),
            username: "admin".to_string(),
            password: Some("super_secret".to_string()),
            remember_password: false,
        };
        profile_no_remember
            .save_to_path(&profile_path)
            .expect("should save profile");

        let loaded =
            ConnectionProfile::load_from_path(&profile_path).expect("should load saved profile");
        assert_eq!(loaded.host, "192.168.1.100");
        assert_eq!(loaded.username, "admin");
        assert!(!loaded.remember_password);
        assert_eq!(loaded.password, None);

        let profile_remember = ConnectionProfile {
            host: "192.168.1.100".to_string(),
            username: "admin".to_string(),
            password: Some("super_secret".to_string()),
            remember_password: true,
        };
        profile_remember
            .save_to_path(&profile_path)
            .expect("should save profile with password");
        let loaded_with_pw =
            ConnectionProfile::load_from_path(&profile_path).expect("should load profile");
        assert_eq!(loaded_with_pw.password.as_deref(), Some("super_secret"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_miniserver_client_check_list_and_download_against_mock_http_server() {
        let xml = "\u{FEFF}<?xml version=\"1.0\" encoding=\"utf-8\"?>\
            <ControlList Version=\"274\">\
                <C Type=\"Document\" U=\"doc-ms\" Title=\"LiveMiniserverHome\" ConfigVersion=\"17020828\"/>\
            </ControlList>";
        let loxcc_bytes = crate::loxcc::compress_loxcc_literals(xml.as_bytes());
        let mut zip_cursor = Cursor::new(Vec::new());
        {
            let mut zw = zip::ZipWriter::new(&mut zip_cursor);
            zw.start_file("sps0.LoxCC", SimpleFileOptions::default())
                .unwrap();
            zw.write_all(&loxcc_bytes).unwrap();
            zw.finish().unwrap();
        }
        let zip_payload = zip_cursor.into_inner();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handle = std::thread::spawn(move || {
            // Serve 3 requests: check_connection, list_snapshots (inside download_snapshot_xml), and fsget
            for _ in 0..3 {
                if let Ok((mut stream, _)) = listener.accept() {
                    let mut buf = [0u8; 2048];
                    let n = stream.read(&mut buf).unwrap_or(0);
                    let req = String::from_utf8_lossy(&buf[..n]);

                    // Verify Basic Auth header for admin:secret ("YWRtaW46c2VjcmV0")
                    if !req.contains("Authorization: Basic YWRtaW46c2VjcmV0") {
                        let resp =
                            "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                        let _ = stream.write_all(resp.as_bytes());
                        continue;
                    }

                    if req.starts_with("GET /dev/fslist/prog ") {
                        let body = "-rw-rw-rw- 1 0 0 158262 Mar 8 18:22 sps_194_20260308182256.zip\r\n";
                        let header = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        );
                        let _ = stream.write_all(header.as_bytes());
                        let _ = stream.write_all(body.as_bytes());
                    } else if req.starts_with("GET /dev/fsget/prog/sps_194_20260308182256.zip ") {
                        let header = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            zip_payload.len()
                        );
                        let _ = stream.write_all(header.as_bytes());
                        let _ = stream.write_all(&zip_payload);
                    }
                }
            }
        });

        let creds = MiniserverCredentials {
            host: format!("http://{}", addr),
            username: "admin".to_string(),
            password: "secret".to_string(),
        };

        let status_info =
            MiniserverClient::check_connection(&creds).expect("check_connection should succeed");
        assert!(status_info.contains("1 backup"));

        let (filename, downloaded_xml) = MiniserverClient::download_snapshot_xml(&creds, None)
            .expect("download_snapshot_xml should succeed");
        assert_eq!(filename, "sps_194_20260308182256.zip");
        assert_eq!(downloaded_xml, xml);

        let _ = server_handle.join();
    }

    #[test]
    fn test_parse_ll_io_response_and_http_polling_and_command() {
        let ll_xml = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\
            <LL control=\"dev/sps/io/1a4f5379-0236-dddf-ffff623289b37d6b/all\" value=\"22.9°\" Code=\"200\">\
                <o name=\"AQ\" value=\"22.9°\"/>\
                <o name=\"AI1\" value=\"22.9\"/>\
            </LL>";
        let snap = parse_ll_io_response(ll_xml).expect("should parse <LL> XML");
        assert_eq!(snap.control_id, "1a4f5379-0236-dddf-ffff623289b37d6b");
        assert_eq!(snap.primary_value.as_deref(), Some("22.9°"));
        assert_eq!(snap.code, 200);
        assert_eq!(snap.outputs.get("AQ").map(|s| s.as_str()), Some("22.9°"));
        assert_eq!(snap.outputs.get("AI1").map(|s| s.as_str()), Some("22.9"));

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handle = std::thread::spawn(move || {
            for _ in 0..2 {
                if let Ok((mut stream, _)) = listener.accept() {
                    let mut buf = [0u8; 2048];
                    let n = stream.read(&mut buf).unwrap_or(0);
                    let req = String::from_utf8_lossy(&buf[..n]);
                    if req.starts_with("GET /dev/sps/io/ctrl-temp/all ") {
                        let body = "<LL control=\"dev/sps/io/ctrl-temp/all\" value=\"23.4°\" Code=\"200\"><o name=\"AQ\" value=\"23.4°\"/></LL>";
                        let header = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: text/xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        );
                        let _ = stream.write_all(header.as_bytes());
                        let _ = stream.write_all(body.as_bytes());
                    } else if req.starts_with("GET /dev/sps/io/ctrl-light/on ") {
                        let body = "<LL control=\"dev/sps/io/ctrl-light/on\" value=\"1\" Code=\"200\"/>";
                        let header = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: text/xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        );
                        let _ = stream.write_all(header.as_bytes());
                        let _ = stream.write_all(body.as_bytes());
                    }
                }
            }
        });

        let creds = MiniserverCredentials {
            host: format!("http://{}", addr),
            username: "admin".to_string(),
            password: "secret".to_string(),
        };

        let polled = MiniserverClient::fetch_io_state(&creds, "ctrl-temp")
            .expect("fetch_io_state should succeed");
        assert_eq!(polled.primary_value.as_deref(), Some("23.4°"));

        let cmd_res = MiniserverClient::send_io_command(&creds, "ctrl-light", "on")
            .expect("send_io_command should succeed");
        assert_eq!(cmd_res.primary_value.as_deref(), Some("1"));

        let _ = server_handle.join();
    }

    #[test]
    fn test_live_value_store_formats_and_propagates_signals_on_pokoj_agatki() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let project = crate::parser::load_loxone_file(&path).expect("Klimczaki.Loxone should parse");
        let page = project
            .pages
            .iter()
            .find(|p| p.title == "Pokój Agatki")
            .expect("Pokój Agatki page should exist");

        let mut store = LiveValueStore::new();
        store.seed_project_defaults(&project);

        // Verify InputRef pills on Pokój Agatki match live.png
        let temp_ref = page
            .blocks
            .iter()
            .find(|b| b.is_input_ref_bar() && b.title == "Temperatura")
            .expect("Temperatura InputRef should exist");
        assert_eq!(ref_bar_capsule_badge(temp_ref, page, &project), "ϑc");
        let temp_sig = store.resolve_ref_bar_signal(temp_ref, page, &project);
        assert_eq!(temp_sig.formatted, "22.9°");
        assert!(temp_sig.is_active);

        let humid_ref = page
            .blocks
            .iter()
            .find(|b| b.is_input_ref_bar() && b.title.contains("Wilgotno"))
            .expect("Wilgotność InputRef should exist");
        assert_eq!(ref_bar_capsule_badge(humid_ref, page, &project), "H");
        let humid_sig = store.resolve_ref_bar_signal(humid_ref, page, &project);
        assert_eq!(humid_sig.formatted, "45%");
        assert!(humid_sig.is_active);

        let lux_ref = page
            .blocks
            .iter()
            .find(|b| b.is_input_ref_bar() && b.title == "Jasność")
            .expect("Jasność InputRef should exist");
        assert_eq!(ref_bar_capsule_badge(lux_ref, page, &project), "Br");
        let lux_sig = store.resolve_ref_bar_signal(lux_ref, page, &project);
        assert_eq!(lux_sig.formatted, "0Lx");
        assert!(!lux_sig.is_active);

        let t5_ref = page
            .blocks
            .iter()
            .find(|b| b.is_input_ref_bar() && b.title == "T5")
            .expect("T5 InputRef should exist");
        assert_eq!(ref_bar_capsule_badge(t5_ref, page, &project), "T5");
        let t5_sig = store.resolve_ref_bar_signal(t5_ref, page, &project);
        assert_eq!(t5_sig.formatted, "0/0/0/0");
        assert!(!t5_sig.is_active);

        // Verify signal propagation into HeatIRoomController2 and out to Zawór Pokój Agatki
        let irc = page
            .blocks
            .iter()
            .find(|b| b.block_type == "HeatIRoomController2")
            .expect("HeatIRoomController2 should exist");
        let temp_pin = irc.pins.iter().find(|p| p.key == "Temp").unwrap();
        let propagated_temp = store.resolve_pin_signal(irc, temp_pin, page, &project);
        assert_eq!(propagated_temp.formatted, "22.9°");
        assert!(propagated_temp.is_active);

        let valve_ref = page
            .blocks
            .iter()
            .find(|b| {
                b.is_output_ref_bar()
                    && (b.title.contains("ogrzewanie") || b.title.contains("Zawór"))
            })
            .expect("Heating valve OutputRef should exist");
        assert_eq!(ref_bar_capsule_badge(valve_ref, page, &project), "VQ");
        let valve_sig = store.resolve_ref_bar_signal(valve_ref, page, &project);
        assert_eq!(valve_sig.formatted, "0.8");
        assert!(valve_sig.is_active);
    }

    #[test]
    fn test_classify_ref_bar_live_control_and_manual_override_on_pokoj_agatki() {
        let path =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let project =
            crate::parser::load_loxone_file(&path).expect("Klimczaki.Loxone should parse");
        let page = project
            .pages
            .iter()
            .find(|p| p.title == "Pokój Agatki")
            .expect("Pokój Agatki page should exist");

        let t5_ref = page
            .blocks
            .iter()
            .find(|b| b.is_input_ref_bar() && b.title == "T5")
            .unwrap();
        assert_eq!(
            classify_ref_bar_live_control(t5_ref, "T5", &project),
            RefBarLiveControlKind::Touch5Pad
        );

        let lux_ref = page
            .blocks
            .iter()
            .find(|b| b.is_input_ref_bar() && b.title == "Jasność")
            .unwrap();
        assert_eq!(
            classify_ref_bar_live_control(lux_ref, "Br", &project),
            RefBarLiveControlKind::NumericValue
        );

        let temp_ref = page
            .blocks
            .iter()
            .find(|b| b.is_input_ref_bar() && b.title == "Temperatura")
            .unwrap();
        assert_eq!(
            classify_ref_bar_live_control(temp_ref, "ϑc", &project),
            RefBarLiveControlKind::NumericValue
        );

        let pres_ref = page
            .blocks
            .iter()
            .find(|b| {
                b.is_input_ref_bar() && ref_bar_capsule_badge(b, page, &project) == "P"
            })
            .unwrap();
        assert_eq!(
            classify_ref_bar_live_control(pres_ref, "P", &project),
            RefBarLiveControlKind::ToggleSwitch
        );

        let move_ref = page
            .blocks
            .iter()
            .find(|b| {
                b.is_input_ref_bar() && ref_bar_capsule_badge(b, page, &project) == "M"
            })
            .unwrap();
        assert_eq!(
            classify_ref_bar_live_control(move_ref, "M", &project),
            RefBarLiveControlKind::Pushbutton
        );

        let mut store = LiveValueStore::new();
        store.seed_project_defaults(&project);

        // Manually change temperature float from 22.9 to 24.6 on temp_ref
        store.set_control_value(&temp_ref.uuid, "24.6");
        let updated_temp = store.resolve_ref_bar_signal(temp_ref, page, &project);
        assert_eq!(updated_temp.formatted, "24.6°");
        assert!(updated_temp.is_active);

        // Manually toggle presence P on pres_ref to 1 ("Wł.") while lux_ref stays 0Lx
        store.set_control_value(&pres_ref.uuid, "1");
        let updated_pres = store.resolve_ref_bar_signal(pres_ref, page, &project);
        assert_eq!(updated_pres.formatted, "Wł.");
        assert!(updated_pres.is_active);
        let unchanged_lux = store.resolve_ref_bar_signal(lux_ref, page, &project);
        assert_eq!(unchanged_lux.formatted, "0Lx");
        assert!(!unchanged_lux.is_active);
    }

    fn read_full_http_request(stream: &mut std::net::TcpStream) -> Vec<u8> {
        let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(2)));
        let mut data = Vec::new();
        let mut chunk = [0u8; 8192];
        let mut expected_total_len: Option<usize> = None;

        loop {
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    data.extend_from_slice(&chunk[..n]);
                    if expected_total_len.is_none() {
                        if let Some(hdr_end) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                            let body_start = hdr_end + 4;
                            let hdr_str = String::from_utf8_lossy(&data[..hdr_end]);
                            let content_len = hdr_str
                                .lines()
                                .find_map(|line| {
                                    let (k, v) = line.split_once(':')?;
                                    if k.trim().eq_ignore_ascii_case("content-length") {
                                        v.trim().parse::<usize>().ok()
                                    } else {
                                        None
                                    }
                                })
                                .unwrap_or(0);
                            expected_total_len = Some(body_start + content_len);
                        }
                    }
                    if let Some(total) = expected_total_len {
                        if data.len() >= total {
                            break;
                        }
                    }
                }
                Err(_) => break,
            }
        }
        data
    }

    #[test]
    fn test_loxone_key_exchange_and_upload_project_with_safety_backup() {
        let json = r#"{"LL":{"control":"jdev/sys/getkey2/admin","value":{"key":"30313233343536373839","salt":"AABBCC","hashAlg":"SHA1"},"Code":"200"}}"#;
        let challenge = parse_getkey2_response(json).expect("should parse getkey2 JSON");
        assert_eq!(challenge.key_hex, "30313233343536373839");
        assert_eq!(challenge.salt_hex, "AABBCC");
        assert_eq!(challenge.hash_alg, "SHA1");

        let auth_hash = compute_loxone_auth_hash("admin", "secret", &challenge);
        assert_eq!(auth_hash.len(), 40);
        assert!(auth_hash.chars().all(|c| c.is_ascii_hexdigit()));

        let old_xml = "\u{FEFF}<?xml version=\"1.0\" encoding=\"utf-8\"?><ControlList Version=\"274\"><C Type=\"Document\" U=\"doc-old\" Title=\"OldRunningConfig\"/></ControlList>";
        let new_xml = "\u{FEFF}<?xml version=\"1.0\" encoding=\"utf-8\"?><ControlList Version=\"274\"><C Type=\"Document\" U=\"doc-new\" Title=\"NewUploadedConfig\"/></ControlList>";
        let old_zip = crate::loxcc::pack_loxcc_zip(old_xml).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let uploaded_zip = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
        let uploaded_zip_clone = uploaded_zip.clone();
        let restart_called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let restart_clone = restart_called.clone();

        let server_handle = std::thread::spawn(move || {
            // 5 requests: fslist, fsget (backup), fsput sps_new.zip, fsput sps0.LoxCC, sps/restart
            for _ in 0..5 {
                if let Ok((mut stream, _)) = listener.accept() {
                    let buf = read_full_http_request(&mut stream);
                    let req = String::from_utf8_lossy(&buf);
                    if req.starts_with("GET /dev/fslist/prog ") {
                        let body = "-rw-rw-rw- 1 0 0 1024 Mar 8 18:22 sps0.LoxCC\r\n";
                        let resp = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            body.len(),
                            body
                        );
                        let _ = stream.write_all(resp.as_bytes());
                    } else if req.starts_with("GET /dev/fsget/prog/sps0.LoxCC ") {
                        let header = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            old_zip.len()
                        );
                        let _ = stream.write_all(header.as_bytes());
                        let _ = stream.write_all(&old_zip);
                    } else if req.starts_with("POST /dev/fsput/prog/sps_new.zip ") {
                        if let Some(idx) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                            let body_bytes = &buf[idx + 4..];
                            *uploaded_zip_clone.lock().unwrap() = body_bytes.to_vec();
                        }
                        let resp = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK";
                        let _ = stream.write_all(resp.as_bytes());
                    } else if req.starts_with("POST /dev/fsput/prog/sps0.LoxCC ") {
                        let resp = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK";
                        let _ = stream.write_all(resp.as_bytes());
                    } else if req.starts_with("GET /dev/sps/restart ") {
                        restart_clone.store(true, std::sync::atomic::Ordering::SeqCst);
                        let resp = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK";
                        let _ = stream.write_all(resp.as_bytes());
                    }
                }
            }
        });

        let temp_backup_dir = std::env::temp_dir().join(format!(
            "loxone_backup_test_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp_backup_dir);

        let creds = MiniserverCredentials {
            host: format!("http://{}", addr),
            username: "admin".to_string(),
            password: "secret".to_string(),
        };

        let result = MiniserverClient::upload_project_xml(
            &creds,
            new_xml,
            Some(&temp_backup_dir),
            true,
        )
        .expect("upload_project_xml should succeed");

        let _ = server_handle.join();

        assert_eq!(result.uploaded_filename, "sps_new.zip");
        assert!(result.restarted_sps);
        assert!(restart_called.load(std::sync::atomic::Ordering::SeqCst));

        let backup_file = result.backup_path.expect("pre-upload backup should be written");
        let saved_backup_xml = std::fs::read_to_string(&backup_file).unwrap();
        assert_eq!(saved_backup_xml, old_xml);

        let received_zip = uploaded_zip.lock().unwrap().clone();
        let unpacked_new = crate::loxcc::extract_loxone_xml(&received_zip).unwrap();
        assert_eq!(unpacked_new, new_xml);

        let _ = std::fs::remove_dir_all(&temp_backup_dir);
    }

    #[test]
    fn test_live_light_toggle_propagates_and_syncs_wires_cleanly() {
        let path =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let project =
            crate::parser::load_loxone_file(&path).expect("Klimczaki.Loxone should parse");
        let page = project
            .pages
            .iter()
            .find(|p| p.title == "Pokój Agatki")
            .expect("Pokój Agatki page should exist");

        let lc_block = page
            .blocks
            .iter()
            .find(|b| b.block_type == "LightController2")
            .expect("LightController2 should exist on Pokój Agatki");
        let aq1_pin = lc_block
            .pins
            .iter()
            .find(|p| p.key == "AQ1")
            .expect("AQ1 pin should exist");
        let aq3_pin = lc_block
            .pins
            .iter()
            .find(|p| p.key == "AQ3")
            .expect("AQ3 pin should exist");

        let light1_out = page
            .blocks
            .iter()
            .find(|b| {
                b.is_output_ref_bar()
                    && b.pins.iter().any(|p| {
                        p.incoming_links
                            .iter()
                            .any(|l| l.source_pin_uuid == aq1_pin.uuid)
                    })
            })
            .expect("agatka: oświetlenie sufit OK OutputRef wired to AQ1 should exist");
        let light3_out = page
            .blocks
            .iter()
            .find(|b| {
                b.is_output_ref_bar()
                    && b.pins.iter().any(|p| {
                        p.incoming_links
                            .iter()
                            .any(|l| l.source_pin_uuid == aq3_pin.uuid)
                    })
            })
            .expect("agatka: podcięcie OK OutputRef wired to AQ3 should exist");

        // Ensure both SMA and Q14 lighting OutputRefs have a ToggleSwitch live control
        let badge1 = ref_bar_capsule_badge(light1_out, page, &project);
        assert_eq!(badge1, "SMA");
        assert_eq!(
            classify_ref_bar_live_control(light1_out, &badge1, &project),
            RefBarLiveControlKind::ToggleSwitch
        );
        let badge3 = ref_bar_capsule_badge(light3_out, page, &project);
        assert_eq!(badge3, "Q14");
        assert_eq!(
            classify_ref_bar_live_control(light3_out, &badge3, &project),
            RefBarLiveControlKind::ToggleSwitch
        );

        let mut store = LiveValueStore::new();
        store.seed_project_defaults(&project);

        // 1. Toggle Light 3 OutputRef ON directly -> both OutputRef and connected AQ3 pin become active ("Wł.")
        store.apply_interactive_command(Some(&project), &light3_out.uuid, None, "1");
        let l3_on = store.resolve_ref_bar_signal(light3_out, page, &project);
        assert!(l3_on.is_active);
        assert_eq!(l3_on.formatted, "Wł.");
        let aq3_on = store.resolve_pin_signal(lc_block, aq3_pin, page, &project);
        assert!(aq3_on.is_active);
        assert_eq!(aq3_on.formatted, "Wł.");
        // Light 1 should NOT be affected
        let l1_still_off = store.resolve_ref_bar_signal(light1_out, page, &project);
        assert!(!l1_still_off.is_active);

        // 2. Toggle Light 3 OutputRef OFF directly -> both OutputRef and connected AQ3 pin become inactive ("Wył.")
        store.apply_interactive_command(Some(&project), &light3_out.uuid, None, "0");
        let l3_off = store.resolve_ref_bar_signal(light3_out, page, &project);
        assert!(!l3_off.is_active);
        assert_eq!(l3_off.formatted, "Wył.");
        let aq3_off = store.resolve_pin_signal(lc_block, aq3_pin, page, &project);
        assert!(!aq3_off.is_active);
        assert_eq!(aq3_off.formatted, "Wył.");

        // 3. Toggle Presence InputRef ("Obecność", badge "P") ON -> propagates through LightController2
        // so AQ1 ("WW(100%)"), AQ3 ("Wł."), and both downstream OutputRefs turn ON!
        let pres_in = page
            .blocks
            .iter()
            .find(|b| b.is_input_ref_bar() && ref_bar_capsule_badge(b, page, &project) == "P")
            .expect("Presence InputRef should exist");
        store.apply_interactive_command(Some(&project), &pres_in.uuid, None, "1");
        let aq1_on = store.resolve_pin_signal(lc_block, aq1_pin, page, &project);
        assert!(aq1_on.is_active);
        assert_eq!(aq1_on.formatted, "WW(100%)");
        assert!(store.resolve_ref_bar_signal(light1_out, page, &project).is_active);
        assert!(store.resolve_pin_signal(lc_block, aq3_pin, page, &project).is_active);
        assert!(store.resolve_ref_bar_signal(light3_out, page, &project).is_active);

        // 4. Toggle Presence InputRef OFF -> all LightController2 outputs and Light OutputRefs turn OFF cleanly!
        store.apply_interactive_command(Some(&project), &pres_in.uuid, None, "0");
        let aq1_off = store.resolve_pin_signal(lc_block, aq1_pin, page, &project);
        assert!(!aq1_off.is_active);
        assert_eq!(aq1_off.formatted, "WW(0%)");
        assert!(!store.resolve_ref_bar_signal(light1_out, page, &project).is_active);
        assert!(!store.resolve_pin_signal(lc_block, aq3_pin, page, &project).is_active);
        assert!(!store.resolve_ref_bar_signal(light3_out, page, &project).is_active);
    }

    #[test]
    fn test_upload_project_xml_safeguard_blocks_newer_server_version_and_succeeds_after_pull_merge() {
        let base_xml = "\u{FEFF}<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n\
<ControlList Version=\"1\">\r\n\
\t<C Type=\"Document\" Title=\"Home\" U=\"doc-1\" ConfigVersion=\"17020828\" CDate=\"2026-10-15 07:37:30\" DateS=\"559780177\" SL=\"271\">\r\n\
\t\t<C Type=\"Program\" Title=\"Program\">\r\n\
\t\t\t<C Type=\"Page\" Title=\"Salon\" U=\"page-1\">\r\n\
\t\t\t\t<C Type=\"Switch\" Title=\"Main Light\" U=\"blk-1\" Px=\"100\" Py=\"200\"/>\r\n\
\t\t\t</C>\r\n\
\t\t</C>\r\n\
\t</C>\r\n\
</ControlList>\r\n";

        let server_newer_xml = "\u{FEFF}<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n\
<ControlList Version=\"1\">\r\n\
\t<C Type=\"Document\" Title=\"Home\" U=\"doc-1\" ConfigVersion=\"17020828\" CDate=\"2026-10-17 15:00:00\" DateS=\"559900000\" SL=\"275\">\r\n\
\t\t<C Type=\"Program\" Title=\"Program\">\r\n\
\t\t\t<C Type=\"Page\" Title=\"Salon\" U=\"page-1\">\r\n\
\t\t\t\t<C Type=\"Switch\" Title=\"Server Priority Light\" U=\"blk-1\" Px=\"100\" Py=\"200\"/>\r\n\
\t\t\t</C>\r\n\
\t\t</C>\r\n\
\t</C>\r\n\
</ControlList>\r\n";

        let server_zip = crate::loxcc::pack_loxcc_zip(server_newer_xml).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let fsput_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let fsput_clone = fsput_count.clone();

        let server_handle = std::thread::spawn(move || {
            // Attempt 1 (blocked after 2 requests: fslist, fsget) + Attempt 2 (succeeds: fslist, fsget, fsput zip, fsput loxcc) = 6 requests
            for _ in 0..6 {
                if let Ok((mut stream, _)) = listener.accept() {
                    let buf = read_full_http_request(&mut stream);
                    let req = String::from_utf8_lossy(&buf);
                    if req.starts_with("GET /dev/fslist/prog ") {
                        let body = "-rw-rw-rw- 1 0 0 1024 Oct 17 15:00 sps_222_20261017150000.zip\r\n";
                        let resp = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            body.len(),
                            body
                        );
                        let _ = stream.write_all(resp.as_bytes());
                    } else if req.starts_with("GET /dev/fsget/prog/sps_222_20261017150000.zip ") {
                        let header = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            server_zip.len()
                        );
                        let _ = stream.write_all(header.as_bytes());
                        let _ = stream.write_all(&server_zip);
                    } else if req.starts_with("POST /dev/fsput/prog/") {
                        fsput_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        let resp = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK";
                        let _ = stream.write_all(resp.as_bytes());
                    }
                }
            }
        });

        let creds = MiniserverCredentials {
            host: format!("http://{}", addr),
            username: "admin".to_string(),
            password: "secret".to_string(),
        };

        // 1. First upload attempt with older local XML (SL=271 vs server SL=275) MUST be blocked!
        let err = MiniserverClient::upload_project_xml_with_safeguard(
            &creds,
            base_xml,
            Some(base_xml),
            None,
            false,
            None,
            false,
        )
        .expect_err("Upload should be blocked by newer server version");

        let conflict = match err {
            MiniserverError::RemoteVersionNewer(c) => c,
            other => panic!("Expected RemoteVersionNewer, got {:?}", other),
        };
        assert_eq!(conflict.diff.local_save_level, Some(271));
        assert_eq!(conflict.diff.remote_save_level, Some(275));
        assert_eq!(fsput_count.load(std::sync::atomic::Ordering::SeqCst), 0);

        // 2. Pull & merge server changes into local project (server priority)
        let mut local_proj = crate::parser::parse_loxone_xml(base_xml).unwrap();
        let base_proj = local_proj.clone();
        let remote_proj = crate::parser::parse_loxone_xml(&conflict.remote_xml).unwrap();
        crate::merge::merge_remote_project_server_priority(
            &mut local_proj,
            &remote_proj,
            Some(&base_proj),
        )
        .unwrap();
        assert_eq!(local_proj.pages[0].blocks[0].title, "Server Priority Light");
        assert_eq!(local_proj.metadata.save_level, Some(275));

        let merged_xml = local_proj.raw_xml.as_deref().unwrap();

        // 3. Second upload attempt with merged project succeeds cleanly!
        let res = MiniserverClient::upload_project_xml_with_safeguard(
            &creds,
            merged_xml,
            Some(&conflict.remote_xml),
            Some(&conflict.remote_xml),
            false,
            None,
            false,
        )
        .expect("Upload should succeed after pulling and merging server version");
        assert_eq!(res.uploaded_filename, "sps_new.zip");
        assert!(fsput_count.load(std::sync::atomic::Ordering::SeqCst) >= 1);

        let _ = server_handle.join();
    }
}
