use std::{
    fs,
    io::{Read, Write},
    path::Path,
    time::{Duration, Instant},
};

use regex::Regex;
use sha2::{Digest, Sha256};
use ureq::{Agent, http::StatusCode};

const READ_BUFFER: usize = 256 * 1024;
const REPORT_INTERVAL: Duration = Duration::from_millis(400);

pub fn agent() -> Agent {
    Agent::config_builder()
        .user_agent(concat!("efi-sk-installer/", env!("CARGO_PKG_VERSION")))
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_per_read(Some(Duration::from_secs(60)))
        // Images are already compressed, and a transparently gzipped body
        // hides Content-Length and breaks byte-range resume.
        .accept_encoding("")
        .http_status_as_error(false)
        .build()
        .into()
}

pub fn fetch_text(agent: &Agent, url: &str) -> Result<String, String> {
    let mut response = agent
        .get(url)
        .call()
        .map_err(|error| format!("Request failed: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("Request failed with HTTP {status}"));
    }
    response
        .body_mut()
        .read_to_string()
        .map_err(|error| format!("Could not read response: {error}"))
}

pub fn download(
    agent: &Agent,
    url: &str,
    path: &Path,
    status: impl Fn(&str),
    mut progress: impl FnMut(u64, Option<u64>),
) -> Result<(), String> {
    let mut last = String::from("Download failed");
    for attempt in 1..=3 {
        match attempt_once(agent, url, path, &mut progress) {
            Ok(()) => return Ok(()),
            Err(error) => {
                last = error;
                if attempt < 3 {
                    status("Download interrupted, retrying...");
                }
            }
        }
    }
    Err(last)
}

fn attempt_once(
    agent: &Agent,
    url: &str,
    path: &Path,
    progress: &mut impl FnMut(u64, Option<u64>),
) -> Result<(), String> {
    let offset = fs::metadata(path).map_or(0, |metadata| metadata.len());
    let mut request = agent.get(url);
    if offset > 0 {
        request = request.header("Range", format!("bytes={offset}-"));
    }
    let mut response = request
        .call()
        .map_err(|error| format!("Request failed: {error}"))?;
    let status = response.status();
    if status == StatusCode::RANGE_NOT_SATISFIABLE && offset > 0 {
        return Ok(());
    }
    if !status.is_success() {
        return Err(format!("Download failed with HTTP {status}"));
    }
    let append = status == StatusCode::PARTIAL_CONTENT;
    let skip = if append { offset } else { 0 };
    let total = response.body().content_length().map(|length| skip + length);
    let mut file = if append {
        fs::OpenOptions::new().append(true).open(path)
    } else {
        fs::File::create(path)
    }
    .map_err(|error| format!("Could not open download file: {error}"))?;
    let mut reader = response.body_mut().as_reader();
    let mut buffer = vec![0; READ_BUFFER];
    let mut received = skip;
    let mut reported = Instant::now();
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| format!("Download failed: {error}"))?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])
            .map_err(|error| format!("Could not save download: {error}"))?;
        received += read as u64;
        if reported.elapsed() >= REPORT_INTERVAL {
            progress(received, total);
            reported = Instant::now();
        }
    }
    progress(received, total);
    Ok(())
}

pub fn expected_digest(sums: &str, pattern: &str) -> Option<String> {
    let pattern = Regex::new(pattern).ok()?;
    let digest = Regex::new(r"[0-9a-fA-F]{64}").ok()?;
    sums.lines().find_map(|line| {
        pattern
            .is_match(line)
            .then(|| {
                digest
                    .find(line)
                    .map(|found| found.as_str().to_ascii_lowercase())
            })
            .flatten()
    })
}

pub fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|error| format!("Could not open file: {error}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("Could not read file: {error}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(text, "{byte:02x}");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn parses_gnu_checksums() {
        let sums = format!("{DIGEST}  image.iso\n");
        assert_eq!(
            expected_digest(&sums, r"image\.iso$").as_deref(),
            Some(DIGEST)
        );
    }

    #[test]
    fn parses_bsd_checksums() {
        let sums = format!("SHA256 (image.iso) = {DIGEST}\n");
        assert_eq!(
            expected_digest(&sums, r"image\.iso\) =").as_deref(),
            Some(DIGEST)
        );
    }

    #[test]
    fn ignores_other_files() {
        let sums = format!("{DIGEST}  other.iso\n");
        assert_eq!(expected_digest(&sums, r"image\.iso$"), None);
    }

    #[test]
    fn ignores_invalid_patterns() {
        assert_eq!(expected_digest("anything", "("), None);
    }

    #[test]
    fn hashes_a_file() {
        let path = std::env::temp_dir().join(format!("efi-sk-sha-{}.txt", std::process::id()));
        fs::write(&path, b"abc").unwrap();
        let digest = sha256_file(&path).unwrap();
        let _ = fs::remove_file(&path);
        assert_eq!(
            digest,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}

#[cfg(test)]
mod network_tests {
    use super::*;

    #[test]
    #[ignore]
    fn fetches_a_small_page() {
        let agent = agent();
        let text = fetch_text(&agent, "https://www.gnu.org/licenses/gpl-3.0.txt").unwrap();
        assert!(text.contains("GNU GENERAL PUBLIC LICENSE"));
        let path = std::env::temp_dir().join(format!("efi-sk-dl-{}", std::process::id()));
        let mut seen = false;
        download(
            &agent,
            "https://www.gnu.org/licenses/gpl-3.0.txt",
            &path,
            |_| {},
            |_, total| {
                seen = seen || total.is_some();
            },
        )
        .unwrap();
        let full = fs::read(&path).unwrap();
        assert!(!full.is_empty());
        assert!(seen);
        let _ = fs::remove_file(path);
    }

    #[test]
    #[ignore]
    fn resumes_a_partial_download() {
        let agent = agent();
        let path = std::env::temp_dir().join(format!("efi-sk-resume-{}", std::process::id()));
        let url = "https://www.gnu.org/licenses/gpl-3.0.txt";
        download(&agent, url, &path, |_| {}, |_, _| {}).unwrap();
        let full = fs::read(&path).unwrap();
        // A truncated file must be appended to, not restarted from zero.
        fs::write(&path, &full[..full.len() / 2]).unwrap();
        download(&agent, url, &path, |_| {}, |_, _| {}).unwrap();
        assert_eq!(fs::read(&path).unwrap(), full);
        let _ = fs::remove_file(path);
    }
}
