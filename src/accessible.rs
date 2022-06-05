use once_cell::sync::OnceCell;

/// Whether to enable accessible output (removes info output and reduces other
/// output, removes visual markers like '[' and ']').
/// Removes th progress bar as well
pub static ACCESSIBLE: OnceCell<bool> = OnceCell::new();

pub fn is_running_in_accessible_mode() -> bool {
    ACCESSIBLE.get().copied().unwrap_or(false)
}

pub fn set_accessible(value: bool) {
    if ACCESSIBLE.get().is_none() {
        ACCESSIBLE.set(value).unwrap();
    }
}
