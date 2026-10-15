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
}
