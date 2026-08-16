use log::{LevelFilter, Log, Metadata, Record};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

struct OpenLog {
    file: File,
    path: PathBuf,
}

struct FileLogger {
    open: Mutex<OpenLog>,
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

impl FileLogger {
    fn write(&self, record: &Record) {
        let Ok(mut open) = self.open.lock() else {
            return;
        };
        let max = jereide_settings::log_max_file_size();
        let rolled = match open.file.metadata() {
            Ok(meta) => meta.len() as usize >= max,
            Err(_) => false,
        };
        if rolled && let Some(next) = open_new_log() {
            *open = next;
            prune_old_logs(jereide_settings::log_max_retention());
        }
        let _ = writeln!(
            open.file,
            "{} [{:<5}] [{}] {}",
            timestamp_now(),
            record.level(),
            record.target(),
            record.args()
        );
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
        if let Ok(open) = self.open.lock() {
            let _ = open.file.sync_all();
        }
    }
}

pub fn log_dir() -> PathBuf {
    jereide_settings::config_dir().join("logs")
}

fn open_new_log() -> Option<OpenLog> {
    let filename = format!("jereide-{}.log", timestamp_now().replace([':', ' '], "-"));
    let path = log_dir().join(filename);
    let file = File::options().append(true).create(true).open(&path).ok()?;
    Some(OpenLog { file, path })
}

fn prune_old_logs(retention: usize) {
    let Ok(entries) = fs::read_dir(log_dir()) else {
        return;
    };
    let mut logs: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy())
                .unwrap_or_default();
            name.starts_with("jereide-") && name.ends_with(".log")
        })
        .collect();
    if logs.len() <= retention {
        return;
    }
    logs.sort();
    let excess = logs.len().saturating_sub(retention);
    for stale in logs.into_iter().take(excess) {
        let _ = fs::remove_file(stale);
    }
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
    let dir = log_dir();
    if let Err(e) = fs::create_dir_all(&dir) {
        eprintln!("jereide-logging: could not create log dir {dir:?}: {e}");
        log::set_max_level(LevelFilter::Off);
        return;
    }
    match open_new_log() {
        Some(open) => {
            let path = open.path.clone();
            let logger = FileLogger {
                open: Mutex::new(open),
            };
            let _ = LOGGER.set(logger);
            if let Err(e) = log::set_logger(LOGGER.get().unwrap()) {
                eprintln!("jereide-logging: could not install logger: {e}");
                return;
            }
            let level = env_max_level();
            log::set_max_level(level);
            prune_old_logs(jereide_settings::log_max_retention());
            log::info!(
                "==== JereIDE logging initialized -> {:?} (level {level}) ====",
                path
            );
        }
        None => {
            eprintln!("jereide-logging: could not open log file");
        }
    }
}

pub fn current_log_path() -> Option<PathBuf> {
    LOGGER
        .get()
        .and_then(|l| l.open.lock().ok().map(|o| o.path.clone()))
}
