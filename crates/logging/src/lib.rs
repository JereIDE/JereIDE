use log::{LevelFilter, Log, Metadata, Record};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const LOG_FILE_NAME: &str = "jereide.log";

struct FileLogger {
    file: Mutex<File>,
    path: PathBuf,
}

static LOGGER: OnceLock<FileLogger> = OnceLock::new();

fn timestamp_now() -> String {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let millis = dur.subsec_millis();
    let mut sod = dur.as_secs();
    let days = (sod / 86_400) as i64;
    sod %= 86_400;
    let h = sod / 3600;
    let mi = (sod % 3600) / 60;
    let s = sod % 60;
    let (y, mo, d) = civil_from_days(days);
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02}.{millis:03}")
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn trim_log_tail(file: &mut File, max_bytes: usize) {
    let len = file.metadata().map(|m| m.len() as usize).unwrap_or(0);
    if len <= max_bytes {
        return;
    }
    let start = len - max_bytes;
    let mut tail = vec![0u8; max_bytes];
    if file.seek(SeekFrom::Start(start as u64)).is_err()
        || file.read_exact(&mut tail).is_err()
        || file.set_len(0).is_err()
        || file.seek(SeekFrom::Start(0)).is_err()
        || file.write_all(&tail).is_err()
    {
        return;
    }
    let _ = file.sync_all();
}

impl FileLogger {
    fn write(&self, record: &Record) {
        let Ok(mut file) = self.file.lock() else {
            return;
        };
        let line = format!(
            "{} [{:<5}] [{}] {}",
            timestamp_now(),
            record.level(),
            record.target(),
            record.args()
        );
        trim_log_tail(&mut file, jereide_settings::log_max_file_size());
        let _ = writeln!(file, "{line}");
    }
}

impl Log for FileLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }
    fn log(&self, record: &Record) {
        self.write(record);
    }
    fn flush(&self) {
        if let Ok(file) = self.file.lock() {
            let _ = file.sync_all();
        }
    }
}

pub fn log_dir() -> PathBuf {
    jereide_settings::config_dir().join("logs")
}

fn env_max_level() -> LevelFilter {
    let Ok(value) = std::env::var("JEREIDE_LOG_LEVEL") else {
        return LevelFilter::Info;
    };
    match value.to_uppercase().as_str() {
        "TRACE" => LevelFilter::Trace,
        "DEBUG" => LevelFilter::Debug,
        "INFO" => LevelFilter::Info,
        "WARN" => LevelFilter::Warn,
        "ERROR" => LevelFilter::Error,
        "OFF" => LevelFilter::Off,
        _ => LevelFilter::Info,
    }
}

pub fn init() {
    if LOGGER.get().is_some() {
        return;
    }
    let path = log_dir().join(LOG_FILE_NAME);
    if let Err(e) = fs::create_dir_all(log_dir()) {
        eprintln!(
            "jereide-logging: could not create log dir {:?}: {e}",
            log_dir()
        );
        log::set_max_level(LevelFilter::Off);
        return;
    }
    let mut file = match File::options().append(true).create(true).open(&path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("jereide-logging: could not open log file {path:?}: {e}");
            return;
        }
    };
    let max = jereide_settings::log_max_file_size();
    if file.metadata().map(|m| m.len() as usize).unwrap_or(0) > max {
        trim_log_tail(&mut file, max);
    }
    let logger = FileLogger {
        file: Mutex::new(file),
        path: path.clone(),
    };
    let _ = LOGGER.set(logger);
    if let Err(e) = log::set_logger(LOGGER.get().unwrap()) {
        eprintln!("jereide-logging: could not install logger: {e}");
        return;
    }
    let level = env_max_level();
    log::set_max_level(level);
    log::info!(
        "==== JereIDE logging initialized -> {:?} (level {level}) ====",
        path
    );
}

pub fn current_log_path() -> Option<PathBuf> {
    LOGGER.get().map(|l| l.path.clone())
}
