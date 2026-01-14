pub mod models;
pub mod services;
pub mod api;

use api::{
    delete_api_key, detect_dual_mode, detect_text, diagnose_api_config, get_api_key, get_config, get_providers,
    preprocess_file, save_config, store_api_key, test_api_connection, get_provider_url, set_provider_url,
    benchmark_gpt_concurrency,
};

use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Instant;
use tracing::info;
use tracing_subscriber::{
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;

static PROCESS_START: OnceLock<Instant> = OnceLock::new();
static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();
static WEBVIEW2_ARGS_SET: OnceLock<String> = OnceLock::new();

fn startup_elapsed_ms() -> u128 {
    PROCESS_START
        .get()
        .map(|t| t.elapsed().as_millis())
        .unwrap_or(0)
}

fn configure_webview_runtime_for_dev() {
    #[cfg(all(windows, debug_assertions))]
    {
        if std::env::var_os("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").is_none() {
            let args = std::env::var("CHEEKAI_WEBVIEW2_ARGS").unwrap_or_else(|_| {
                // Prevent WebView2 from being affected by system proxy / WPAD delays when loading the local dev server.
                "--no-proxy-server --proxy-bypass-list=<-loopback>".to_string()
            });

            std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", &args);
            let _ = WEBVIEW2_ARGS_SET.set(args);
        }
    }
}

#[tauri::command]
fn report_frontend_ready(phase: Option<String>, client_ms: Option<f64>) -> Result<(), String> {
    info!(
        startup_ms = startup_elapsed_ms(),
        phase = phase.as_deref().unwrap_or("unknown"),
        client_ms = client_ms.unwrap_or(-1.0),
        "frontend.ready"
    );
    Ok(())
}

/// Initialize logging system with timestamped log files
fn init_logging() {
    let disable_file_log = matches!(
        std::env::var("CHEEKAI_DISABLE_FILE_LOG").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE")
    );
    let disable_cleanup = matches!(
        std::env::var("CHEEKAI_DISABLE_LOG_CLEANUP").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE")
    );

    // Configure subscriber filter as early as possible (so fallback logging is consistent).
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    if disable_file_log {
        init_console_only_logging(env_filter);
        info!("File logging disabled via CHEEKAI_DISABLE_FILE_LOG");
        return;
    }

    // Get logs directory path (next to executable or in app data)
    let logs_dir = match std::env::var("CHEEKAI_LOG_DIR") {
        Ok(p) if !p.trim().is_empty() => PathBuf::from(p),
        _ => get_logs_dir(),
    };
    
    // Ensure logs directory exists
    if let Err(e) = fs::create_dir_all(&logs_dir) {
        eprintln!("Failed to create logs directory: {}", e);
        init_console_only_logging(env_filter);
        info!("Falling back to console-only logging (log dir not writable)");
        return;
    }
    
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let log_filename = format!("cheekAI_{}.log", timestamp);

    // Create a dedicated file per session; keep log writes non-blocking.
    let file_appender = rolling::never(&logs_dir, &log_filename);
    let (file_writer, file_guard) = tracing_appender::non_blocking(file_appender);
    let _ = LOG_GUARD.set(file_guard);
    
    let file_layer = fmt::layer()
        .with_writer(file_writer)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true);
    
    #[cfg(debug_assertions)]
    {
        // Console layer for development
        let console_layer = fmt::layer()
            .with_writer(std::io::stdout)
            .with_ansi(true)
            .with_target(true);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(file_layer)
            .with(console_layer)
            .init();
    }

    #[cfg(not(debug_assertions))]
    {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(file_layer)
            .init();
    }
    
    info!("=== CheekAI Started ===");
    info!("Log file: {}/{}", logs_dir.display(), log_filename);
    info!("Version: {}", env!("CARGO_PKG_VERSION"));

    // Best-effort cleanup in the background (avoid adding startup I/O latency).
    if !disable_cleanup {
        std::thread::spawn(move || {
            cleanup_old_logs(&logs_dir, 30);
        });
    }
}

/// Get the logs directory path
fn get_logs_dir() -> PathBuf {
    // Development: use current directory
    // Production: use app data directory
    #[cfg(debug_assertions)]
    {
        // Dev mode: always write logs to the repo root `logs/` instead of `src-tauri/logs`.
        // This keeps logs consistent during development regardless of the current working directory.
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("logs")
    }
    
    #[cfg(not(debug_assertions))]
    {
        if let Some(data_dir) = dirs::data_local_dir() {
            return data_dir.join("cheekAI").join("logs");
        }
        PathBuf::from("logs")
    }
}

fn cleanup_old_logs(logs_dir: &PathBuf, keep: usize) {
    let mut entries: Vec<_> = match fs::read_dir(logs_dir) {
        Ok(rd) => rd.filter_map(|e| e.ok()).collect(),
        Err(_) => return,
    };

    entries.retain(|e| {
        let name = e.file_name().to_string_lossy().to_string();
        name.starts_with("cheekAI_") && name.ends_with(".log")
    });

    if entries.len() <= keep {
        return;
    }

    entries.sort_by_key(|e| {
        e.metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    });

    let remove_count = entries.len().saturating_sub(keep);
    for entry in entries.into_iter().take(remove_count) {
        let _ = fs::remove_file(entry.path());
    }
}

fn init_console_only_logging(env_filter: EnvFilter) {
    #[cfg(debug_assertions)]
    {
        let console_layer = fmt::layer()
            .with_writer(std::io::stdout)
            .with_ansi(true)
            .with_target(true);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(console_layer)
            .init();
    }

    #[cfg(not(debug_assertions))]
    {
        let console_layer = fmt::layer()
            .with_writer(std::io::stderr)
            .with_ansi(false)
            .with_target(true);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(console_layer)
            .init();
    }
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    PROCESS_START.get_or_init(Instant::now);
    configure_webview_runtime_for_dev();

    // Initialize logging system
    let logging_t0 = Instant::now();
    init_logging();
    info!(startup_ms = startup_elapsed_ms(), logging_ms = logging_t0.elapsed().as_millis(), "logging.initialized");
    if let Some(args) = WEBVIEW2_ARGS_SET.get() {
        info!(args = %args, "webview2.additional_browser_arguments_set");
    }
    
    info!("Initializing Tauri application...");
    
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|_app| {
            info!(startup_ms = startup_elapsed_ms(), "tauri.setup");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            detect_text,
            detect_dual_mode,
            get_config,
            save_config,
            get_providers,
            store_api_key,
            get_api_key,
            delete_api_key,
            get_provider_url,
            set_provider_url,
            preprocess_file,
            benchmark_gpt_concurrency,
            diagnose_api_config,
            test_api_connection,
            report_frontend_ready,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                info!("Window close requested: {}", window.label());
            }
            if let tauri::WindowEvent::Destroyed = event {
                info!("=== CheekAI Shutting Down ===");
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
    
    info!("=== CheekAI Exited ===");
}
