//! Test helpers shared across backend modules.

use std::sync::OnceLock;

use tauri::Manager;

/// Returns a reusable test app handle built with the real desktop runtime.
///
/// The handle is cached so backend tests can share a single initialized app
/// without repeatedly constructing the runtime.
#[must_use]
pub fn mock_app_handle() -> tauri::AppHandle {
    static APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();

    APP_HANDLE
        .get_or_init(|| {
            tauri::Builder::default()
                .any_thread()
                .build(tauri::generate_context!())
                .expect("failed to build test app")
                .handle()
                .clone()
        })
        .clone()
}

/// Wraps a borrowed test fixture in Tauri's `State` guard for command tests.
///
/// This is test-only plumbing to keep legacy command tests working without
/// forcing them to spin up a managed application state for every case.
#[must_use]
pub fn state<T>(value: &T) -> tauri::State<'static, T>
where
    T: Clone + Send + Sync + 'static,
{
    let app = tauri::test::mock_app();
    assert!(tauri::Manager::manage(&app, value.clone()));
    let app: &'static tauri::App<_> = Box::leak(Box::new(app));
    app.state::<T>()
}
