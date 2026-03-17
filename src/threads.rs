use std::fs;

/// Returns a list of all thread IDs (TIDs) for a given process ID (PID)
/// This function reads the /proc/[PID]/task/ directory, which contains a subdirectory for each thread of the process.
// For example, if PID 1234 has 3 threads, there will be directories:
// /proc/1234/task/1111/
// /proc/1234/task/2222/
// /proc/1234/task/3333/
pub fn get_tids(pid: u32) -> Vec<u32> {
    let mut tids = Vec::new();
    let path = format!("/proc/{}/task/", pid);
    // Read the directory entries
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(file_name) = entry.file_name().into_string() {
                if let Ok(tid) = file_name.parse::<u32>() {
                    tids.push(tid);
                }
            }
        }
    }
    tids.sort_unstable();
    tids
}
