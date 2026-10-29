//! UI-only sidecar ownership. Never adopt a daemon from its PID file alone and
//! never substitute an in-process library observation for an OS child.
use eltor::{foundation_status_bridge::FoundationStatusBridge, FoundationClientStatus};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::{Child, Command},
    sync::{Arc, LazyLock, Mutex},
    time::Duration,
};

struct OwnedChild {
    child: Arc<Mutex<Child>>,
    bridge: FoundationStatusBridge,
}
static CHILDREN: LazyLock<Mutex<BTreeMap<PathBuf, OwnedChild>>> =
    LazyLock::new(|| Mutex::new(BTreeMap::new()));

pub(super) fn prepare(
    client: bool,
    pid_file: &Path,
    command: &mut Command,
) -> Option<FoundationStatusBridge> {
    use eltor::foundation_status_bridge::{STATUS_DIRECTORY_ENV, STATUS_GENERATION_ENV};
    // Only this launch may supply its private channel. Never inherit another
    // parent's status endpoint through a managed backend or environment file.
    command
        .env_remove(STATUS_DIRECTORY_ENV)
        .env_remove(STATUS_GENERATION_ENV);
    if !client || !configured() {
        return None;
    }
    match pid_file
        .parent()
        .and_then(|p| FoundationStatusBridge::create(p).ok())
    {
        Some(bridge) => {
            bridge.configure_child(command);
            Some(bridge)
        }
        None => {
            log::warn!("Foundation sidecar observation unavailable; UI will require recovery");
            None
        }
    }
}
fn configured() -> bool {
    std::env::var_os("ELTOR_FOUNDATION_CLIENT_PLAN").is_some()
        || std::env::var_os("ELTOR_FOUNDATION_CONFIG").is_some()
}
pub(super) fn spawned(pid_file: PathBuf, mut child: Child, bridge: Option<FoundationStatusBridge>) {
    let Some(bridge) = bridge else {
        std::thread::spawn(move || {
            let _ = child.wait();
        });
        return;
    };
    let child = Arc::new(Mutex::new(child));
    if let Ok(mut children) = CHILDREN.lock() {
        if children.len() < 2 || children.contains_key(&pid_file) {
            children.insert(
                pid_file.clone(),
                OwnedChild {
                    child: child.clone(),
                    bridge,
                },
            );
        }
    }
    // Retain and reap the actual Child, not a guessed PID generation. No
    // observation thread may issue Tor, wallet or payment operations.
    std::thread::spawn(move || loop {
        let exited = {
            child
                .lock()
                .map(|mut c| !matches!(c.try_wait(), Ok(None)))
                .unwrap_or(true)
        };
        if exited {
            if let Ok(mut children) = CHILDREN.lock() {
                if children
                    .get(&pid_file)
                    .is_some_and(|entry| Arc::ptr_eq(&entry.child, &child))
                {
                    children.remove(&pid_file);
                }
            }
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    });
}
pub(super) fn forget(pid_file: &Path) {
    if let Ok(mut children) = CHILDREN.lock() {
        children.remove(pid_file);
    }
}
pub(super) fn observed(client_running: bool, pid_file: &Path) -> Option<FoundationClientStatus> {
    if !client_running || !configured() {
        return None;
    }
    Some(read_owned(pid_file).unwrap_or(FoundationClientStatus::Recovering))
}
fn read_owned(pid_file: &Path) -> Option<FoundationClientStatus> {
    let children = CHILDREN.lock().ok()?;
    let entry = children.get(pid_file)?;
    let mut child = entry.child.lock().ok()?;
    if !matches!(child.try_wait(), Ok(None)) {
        return None;
    }
    // Existing PID files are bookkeeping, never authority. A changed file must
    // agree with the exact Child before its bound observation can be displayed.
    use std::io::Read;
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(pid_file).ok()?;
    let metadata = file.metadata().ok()?;
    if !metadata.is_file() || metadata.len() > 16 {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(17).read_to_end(&mut bytes).ok()?;
    if bytes.len() > 16 {
        return None;
    }
    let text = std::str::from_utf8(&bytes).ok()?;
    let saved: u32 = text.parse().ok()?;
    if text != saved.to_string() {
        return None;
    }
    if saved != child.id() {
        return None;
    }
    Some(entry.bridge.read(child.id()))
}
