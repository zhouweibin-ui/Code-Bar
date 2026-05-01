use std::path::PathBuf;

use crate::util::home_dir;

fn push_unique_path(paths: &mut Vec<PathBuf>, candidate: PathBuf) {
    if paths.iter().any(|existing| existing == &candidate) {
        return;
    }
    paths.push(candidate);
}

pub(crate) fn codex_sessions_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    for provider_dir in crate::util::provider_dir_candidates("codex", "") {
        push_unique_path(&mut dirs, provider_dir.join("sessions"));
    }

    if let Some(home) = home_dir() {
        push_unique_path(&mut dirs, home.join(".codex").join("sessions"));
    }

    dirs
}
