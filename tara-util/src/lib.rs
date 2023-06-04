#![feature(let_chains)]

pub mod error;
/// Interprocess communication functions and data structures for the client and server.
pub mod ipc;
pub mod logging;
pub mod paths;

/// Gets the number of instances of the current process
///
/// # Panics
///
/// This will never panic.
pub fn current_process_instance_count() -> std::io::Result<usize> {
    use sysinfo::SystemExt;
    let process_name = std::env::current_exe()?
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let mut system = sysinfo::System::new();
    system.refresh_processes();

    Ok(system.processes_by_exact_name(&process_name).count())
}
