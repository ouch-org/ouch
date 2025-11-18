use std::sync::OnceLock;

static USER_DEFINED_THREAD_COUNT: OnceLock<usize> = OnceLock::new();

pub fn logical_thread_count() -> usize {
    USER_DEFINED_THREAD_COUNT.get().copied().unwrap_or(num_cpus::get())
}

pub fn physical_thread_count() -> usize {
    USER_DEFINED_THREAD_COUNT
        .get()
        .copied()
        .unwrap_or(num_cpus::get_physical())
}

pub fn set_thread_count(value: usize) {
    if USER_DEFINED_THREAD_COUNT.get().is_none() {
        USER_DEFINED_THREAD_COUNT.set(value).unwrap();
    }
}
