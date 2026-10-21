uniffi::setup_scaffolding!();

#[uniffi::export]
pub fn hello_from_rust() -> String {
    shared::hello_core().to_string()
}