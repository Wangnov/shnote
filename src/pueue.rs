use std::path::PathBuf;

use which::which;

use crate::config::{pueue_binary_name, pueued_binary_name, shnote_bin_dir};

fn find_in_shnote_bin(binary_name: &str) -> Option<PathBuf> {
    shnote_bin_dir()
        .ok()
        .map(|dir| dir.join(binary_name))
        .filter(|path| path.exists())
}

pub fn find_pueue() -> Option<PathBuf> {
    find_in_shnote_bin(pueue_binary_name()).or_else(|| which("pueue").ok())
}

pub fn find_pueued() -> Option<PathBuf> {
    find_in_shnote_bin(pueued_binary_name()).or_else(|| which("pueued").ok())
}

pub fn pueue_available() -> bool {
    find_pueue().is_some() && find_pueued().is_some()
}
