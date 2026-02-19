//! Funtions to obtain and cache start time of processes.

use chrono::TimeDelta;
use procfs::process::Process;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

// TODO: perhaps we can move mutex to ProcessUptimeCollector.cache I think, or not?
static GLOBAL_INSTANCE: LazyLock<Mutex<ProcessUptimeCollector>> =
    LazyLock::new(|| Mutex::new(ProcessUptimeCollector::new()));

pub fn get_process_start_time(pid: i32) -> std::option::Option<chrono::DateTime<chrono::Local>> {
    GLOBAL_INSTANCE.lock().unwrap().get_process_start_time(pid)
}

struct ProcessUptimeCollector {
    cache: RefCell<HashMap<i32, chrono::DateTime<chrono::Local>>>,
    clock_ticks_per_sec: u32,
    sys_start_time: chrono::DateTime<chrono::Local>,
}

impl ProcessUptimeCollector {
    fn new() -> Self {
        let mut tps = u32::try_from(rustix::param::clock_ticks_per_second()).unwrap_or_default();
        if tps == 0 {
            tps = 1; // preventing div by zero if we don't have ctps info.
        }

        let sys_start_time = procfs::boot_time().unwrap();

        Self {
            cache: HashMap::default().into(),
            clock_ticks_per_sec: tps,
            sys_start_time,
        }
    }

    fn get_process_start_time(&self, pid: i32) -> Option<chrono::DateTime<chrono::Local>> {
        if let Some(time) = self.cache.borrow().get(&pid) {
            return Some(*time);
        }
        match _get_process_start_time(pid, self.clock_ticks_per_sec, self.sys_start_time) {
            Ok(x) => {
                self.cache.borrow_mut().insert(pid, x);
                Some(x)
            }
            Err(e) => {
                println!("get_process_start_time: {:?}", e);
                None
            }
        }
    }
}

fn _get_process_start_time(
    pid: i32,
    clock_ticks_per_sec: u32,
    sys_start_time: chrono::DateTime<chrono::Local>,
) -> anyhow::Result<chrono::DateTime<chrono::Local>> {
    // Create a new Process instance for the given PID
    let process = Process::new(pid).map_err(|e| {
        io::Error::new(io::ErrorKind::NotFound, format!("Process not found: {}", e))
    })?;

    // Retrieve the process statistics
    let stat = process
        .stat()
        .map_err(|e| io::Error::other(format!("Failed to get process stat: {}", e)))?;

    let started_since_boot = TimeDelta::from_std(Duration::from_secs(
        stat.starttime / (clock_ticks_per_sec as u64),
    ))?;

    let ret = sys_start_time
        .checked_add_signed(started_since_boot)
        .ok_or_else(|| anyhow::anyhow!("checked_add failed"))?;

    Ok(ret)
}
