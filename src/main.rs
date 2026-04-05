mod http;
mod migration;
mod rtdata;
mod settings;
mod tls;

use std::{
    env,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

const DEFAULT_SETTINGS_YAML: &str = include_str!("../settings-default.yaml");

use log::info;
use rcgen::{generate_simple_self_signed, CertifiedKey};
use tokio::runtime::{Builder, Runtime};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use http::run_http_server;
use settings::{SettingSpec, Settings};
use tls::ServerTlsConfig;

#[cfg(target_os = "macos")]
use tao::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
#[cfg(target_os = "macos")]
use webbrowser;
#[cfg(target_os = "macos")]
use cocoa::{
    appkit::{NSApp, NSBezelStyle},
    base::{id, nil},
    foundation::{NSString, NSRect, NSPoint, NSSize},
};
#[cfg(target_os = "macos")]
use objc::{
    class, msg_send,
    runtime::{Class, Object, Sel},
    sel, sel_impl,
};
#[cfg(target_os = "macos")]
use std::os::raw::c_void;
#[cfg(target_os = "macos")]
use tao::platform::macos::WindowExtMacOS;
#[cfg(target_os = "macos")]
use open;

#[derive(Clone)]
struct RuntimeConfig {
    host: String,
    port: u16,
    data_dir: PathBuf,
    metrics_dir: Option<PathBuf>,
    flush_ms: u64,
    auth_enabled: bool,
    auth_secret_bytes: Option<Vec<u8>>,
    server_tls: Option<ServerTlsConfig>,
    tls_enabled: bool,
    config_path: Option<PathBuf>,
}

struct ServerRuntime {
    runtime: Runtime,
    handle: JoinHandle<Result<(), String>>,
    ready_rx: Option<oneshot::Receiver<Result<(), String>>>,
}

fn generate_self_signed_cert(out_dir: &Path) -> (PathBuf, PathBuf) {
    fs::create_dir_all(out_dir).unwrap();

    // Simple self-signed certificate for local usage
    let cert: CertifiedKey = generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    let cert_pem = cert.cert.pem();
    let key_pem = cert.key_pair.serialize_pem();

    let cert_path = out_dir.join("server.crt");
    let key_path = out_dir.join("server.key");

    fs::write(&cert_path, cert_pem).unwrap();
    fs::write(&key_path, key_pem).unwrap();

    (cert_path, key_path)
}

#[cfg(target_os = "macos")]
fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let user_config_arg = parse_config_arg();
    let in_bundle = running_in_app_bundle();
    let config = load_runtime_config(user_config_arg.clone(), in_bundle);
    let service_url = build_service_url(&config.host, config.port, config.tls_enabled);

    let mut server_runtime = spawn_server(&config);

    wait_for_server_ready(&mut server_runtime);

    if in_bundle {
        launch_window(user_config_arg, in_bundle, service_url, config, server_runtime);
    } else {
        let ServerRuntime { runtime, handle, .. } = server_runtime;
        runtime.block_on(async move {
            match handle.await {
                Ok(Ok(())) => {}
                Ok(Err(err)) => eprintln!("Server error: {err}"),
                Err(err) => eprintln!("Server task failed: {err}"),
            }
        });
    }
}

#[cfg(not(target_os = "macos"))]
#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let config_path = parse_config_arg();
    let config = load_runtime_config(config_path, false);
    run_server(config, None)
        .await
        .expect("server failed to start");
}

fn load_runtime_config(config_path: Option<PathBuf>, in_bundle: bool) -> RuntimeConfig {
    let cwd = env::current_dir().expect("failed to get cwd");

    let default_data_dir = if in_bundle {
        #[cfg(target_os = "macos")]
        {
            mac_default_data_dir().unwrap_or_else(|| cwd.join("data_dir"))
        }
        #[cfg(not(target_os = "macos"))]
        {
            cwd.join("data_dir")
        }
    } else {
        cwd.join("data_dir")
    };

    let resolved_config = config_path.or_else(|| resolve_default_config(in_bundle));

    let settings = Settings::from_optional_file(resolved_config.as_ref()).unwrap_or_else(|e| {
        eprintln!("Failed to load settings: {e}");
        std::process::exit(1);
    });

    let host: String = settings
        .value(&SettingSpec {
            section: "server",
            key: "bind_host",
            env_var: "BIND_HOST",
            default: "0.0.0.0".to_string(),
        })
        .expect("failed to read host");

    let port: u16 = settings
        .value(&SettingSpec {
            section: "server",
            key: "bind_port",
            env_var: "BIND_PORT",
            default: 8245u16,
        })
        .expect("failed to read port");

    let data_dir: PathBuf = settings
        .value(&SettingSpec {
            section: "paths",
            key: "data_dir",
            env_var: "DATA_DIR",
            default: default_data_dir.clone(),
        })
        .expect("failed to read data_dir");

    let tls_enabled: bool = settings
        .value(&SettingSpec {
            section: "tls",
            key: "enabled",
            env_var: "TLS_ENABLE",
            default: false,
        })
        .expect("failed to read tls.enabled");

    let tls_auto: bool = settings
        .value(&SettingSpec {
            section: "tls",
            key: "auto",
            env_var: "TLS_AUTO",
            default: true,
        })
        .expect("failed to read tls.auto");

    let tls_cert_path: Option<PathBuf> = settings
        .value(&SettingSpec {
            section: "tls",
            key: "cert_path",
            env_var: "TLS_CERT_PATH",
            default: None::<PathBuf>,
        })
        .expect("failed to read tls.cert_path");

    let tls_key_path: Option<PathBuf> = settings
        .value(&SettingSpec {
            section: "tls",
            key: "key_path",
            env_var: "TLS_KEY_PATH",
            default: None::<PathBuf>,
        })
        .expect("failed to read tls.key_path");

    let metrics_dir: Option<PathBuf> = settings
        .value(&SettingSpec {
            section: "metrics",
            key: "dir",
            env_var: "METRICS_DIR",
            default: None::<PathBuf>,
        })
        .expect("failed to read metrics.dir");

    let flush_ms: u64 = settings
        .value(&SettingSpec {
            section: "persistence",
            key: "flush_ms",
            env_var: "PERSIST_FLUSH_MS",
            default: 15_000u64,
        })
        .expect("failed to read persistence.flush_ms");

    let auth_enabled: bool = settings
        .value(&SettingSpec {
            section: "auth",
            key: "enabled",
            env_var: "AUTH_ENABLED",
            default: false,
        })
        .expect("failed to read auth.enabled");

    let auth_secret_str: Option<String> = settings
        .value(&SettingSpec {
            section: "auth",
            key: "secret",
            env_var: "AUTH_SECRET",
            default: None::<String>,
        })
        .expect("failed to read auth.secret");

    let auth_secret_bytes = auth_secret_str.map(|s| s.into_bytes());

    let server_tls = if tls_enabled {
        let (cert_path, key_path) = match (tls_cert_path, tls_key_path, tls_auto) {
            (Some(cert_path), Some(key_path), _) => (cert_path, key_path),
            (None, None, true) => generate_self_signed_cert(&data_dir.join("certificates")),
            (None, None, false) => {
                eprintln!("TLS is enabled but no cert/key provided and TLS_AUTO=false; provide TLS_CERT_PATH and TLS_KEY_PATH or enable TLS_AUTO.");
                std::process::exit(1);
            }
            (Some(_), None, _) | (None, Some(_), _) => {
                eprintln!("Both TLS_CERT_PATH and TLS_KEY_PATH must be provided when TLS is enabled.");
                std::process::exit(1);
            }
        };
        Some(ServerTlsConfig::new(cert_path, key_path))
    } else {
        None
    };

    RuntimeConfig {
        host,
        port,
        data_dir,
        metrics_dir,
        flush_ms,
        auth_enabled,
        auth_secret_bytes,
        server_tls,
        tls_enabled,
        config_path: resolved_config,
    }
}

async fn run_server(
    config: RuntimeConfig,
    ready_tx: Option<oneshot::Sender<Result<(), String>>>,
) -> Result<(), String> {
    let rt_db_dir = config.data_dir.join("rt_data");
    let static_db_file = config.data_dir.join("static.db");
    fs::create_dir_all(&rt_db_dir)
        .map_err(|e| format!("Failed to create data dir {}: {e}", rt_db_dir.display()))?;

    let http_tls = config.server_tls.clone();
    let http_schema = if http_tls.is_some() { "https" } else { "http" };
    let ws_schema = if http_tls.is_some() { "wss" } else { "ws" };
    info!("Starting unified server on {http_schema}/{ws_schema}://{}:{}", config.host, config.port);

    run_http_server(
        &config.host,
        config.port,
        &rt_db_dir,
        &static_db_file,
        http_tls,
        config.metrics_dir.clone(),
        config.flush_ms,
        config.auth_enabled,
        config.auth_secret_bytes.clone(),
        ready_tx,
    )
    .await
}

fn spawn_server(config: &RuntimeConfig) -> ServerRuntime {
    let runtime = Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to create tokio runtime");
    let config_clone = config.clone();
    let (ready_tx, ready_rx) = oneshot::channel();
    let handle = runtime.spawn(async move { run_server(config_clone, Some(ready_tx)).await });
    ServerRuntime {
        runtime,
        handle,
        ready_rx: Some(ready_rx),
    }
}

fn parse_config_arg() -> Option<PathBuf> {
    let mut args = env::args().skip(1);
    let mut config: Option<PathBuf> = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" | "-c" => {
                if let Some(path) = args.next() {
                    config = Some(PathBuf::from(path));
                } else {
                    eprintln!("--config <path> requires a file path");
                    std::process::exit(2);
                }
            }
            _ if arg.starts_with("--config=") => {
                let (_, path) = arg.split_at("--config=".len());
                if path.is_empty() {
                    eprintln!("--config=<path> requires a file path");
                    std::process::exit(2);
                }
                config = Some(PathBuf::from(path));
            }
            _ if arg.starts_with("-c=") => {
                let (_, path) = arg.split_at("-c=".len());
                if path.is_empty() {
                    eprintln!("-c=<path> requires a file path");
                    std::process::exit(2);
                }
                config = Some(PathBuf::from(path));
            }
            unknown => {
                eprintln!("Unknown argument: {unknown}");
                std::process::exit(2);
            }
        }
    }
    config
}

fn build_service_url(host: &str, port: u16, tls_enabled: bool) -> String {
    let scheme = if tls_enabled { "https" } else { "http" };
    let browser_host = if host == "0.0.0.0" { "localhost" } else { host };
    format!("{scheme}://{browser_host}:{port}")
}

fn resolve_default_config(in_bundle: bool) -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let app_support = mac_default_data_dir().map(|dir| dir.join("settings.yaml"));
        let bundle_cfg = if in_bundle {
            bundle_resources_dir().map(|r| r.join("settings.yaml"))
        } else {
            None
        };

        if let Some(cfg) = app_support.as_ref().filter(|p| p.exists()) {
            return Some(cfg.clone());
        }

        if let (Some(app_cfg), Some(bundle_cfg)) = (app_support.clone(), bundle_cfg) {
            if bundle_cfg.exists() {
                if let Some(parent) = app_cfg.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if fs::copy(&bundle_cfg, &app_cfg).is_ok() {
                    return Some(app_cfg);
                }
                return Some(bundle_cfg);
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let etc_cfg = PathBuf::from("/etc/lirays-scada/settings.yaml");
        if etc_cfg.exists() {
            return Some(etc_cfg);
        }
    }

    None
}

#[cfg(target_os = "macos")]
fn wait_for_server_ready(server_runtime: &mut ServerRuntime) {
    use std::time::Duration;

    let Some(mut rx) = server_runtime.ready_rx.take() else {
        return;
    };

    let mut waited_ms = 0u64;
    loop {
        match rx.try_recv() {
            Ok(Ok(())) => break,
            Ok(Err(err)) => {
                eprintln!("Service failed to start: {err}");
                std::process::exit(1);
            }
            Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {
                std::thread::sleep(Duration::from_millis(25));
                waited_ms += 25;
            }
            Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                eprintln!("Service failed to signal readiness");
                break;
            }
        }
        if waited_ms > 5000 {
            eprintln!("Service did not signal readiness within 5s");
            break;
        }
    }
}

#[cfg(target_os = "macos")]
fn running_in_app_bundle() -> bool {
    env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().and_then(|p| p.parent()).and_then(|p| p.parent()).map(PathBuf::from))
        .and_then(|bundle| bundle.extension().map(|ext| ext == "app"))
        .unwrap_or(false)
}

#[cfg(target_os = "macos")]
fn mac_default_data_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join("Library/Application Support/LiRays-Scada"))
}

#[cfg(target_os = "macos")]
fn bundle_resources_dir() -> Option<PathBuf> {
    env::current_exe().ok().and_then(|exe| {
        exe.parent()
            .and_then(|p| p.parent()) // Contents
            .map(|contents| contents.join("Resources"))
    })
}

#[cfg(target_os = "macos")]
unsafe fn setup_cocoa_ui(window: &tao::window::Window, state: &mut UiState, action_tx: std::sync::mpsc::Sender<UiAction>) {
    let ns_window: id = window.ns_window() as id;

    // Ensure the app activates
    let app = NSApp();
    let _: () = msg_send![app, activateIgnoringOtherApps: true];

    let content_view: id = msg_send![ns_window, contentView];

    // Title label
    let title: id = msg_send![class!(NSTextField), alloc];
    let title_frame = NSRect::new(NSPoint::new(20.0, 170.0), NSSize::new(460.0, 24.0));
    let title: id = msg_send![title, initWithFrame: title_frame];
    let title_str = NSString::alloc(nil).init_str("LiRays SCADA");
    let _: () = msg_send![title, setStringValue: title_str];
    let _: () = msg_send![title, setEditable: false];
    let _: () = msg_send![title, setBezeled: false];
    let _: () = msg_send![title, setDrawsBackground: false];
    let _: () = msg_send![content_view, addSubview: title];

    // URL label
    let url_label: id = msg_send![class!(NSTextField), alloc];
    let url_frame = NSRect::new(NSPoint::new(20.0, 140.0), NSSize::new(460.0, 22.0));
    let url_label: id = msg_send![url_label, initWithFrame: url_frame];
    set_label_value(url_label, &state.service_url);
    let _: () = msg_send![url_label, setEditable: false];
    let _: () = msg_send![url_label, setBezeled: false];
    let _: () = msg_send![url_label, setDrawsBackground: false];
    let _: () = msg_send![content_view, addSubview: url_label];
    state.url_label = Some(url_label);

    let target = create_action_target(action_tx);

    // Buttons
    let open_cfg_btn = make_button(
        NSRect::new(NSPoint::new(20.0, 90.0), NSSize::new(200.0, 28.0)),
        "Abrir settings.yaml",
        sel!(handleOpenConfig:),
        target,
    );
    let restart_btn = make_button(
        NSRect::new(NSPoint::new(240.0, 90.0), NSSize::new(150.0, 28.0)),
        "Reiniciar servicio",
        sel!(handleRestart:),
        target,
    );
    let open_browser_btn = make_button(
        NSRect::new(NSPoint::new(20.0, 50.0), NSSize::new(200.0, 28.0)),
        "Abrir en navegador",
        sel!(handleOpenBrowser:),
        target,
    );

    let _: () = msg_send![content_view, addSubview: open_cfg_btn];
    let _: () = msg_send![content_view, addSubview: restart_btn];
    let _: () = msg_send![content_view, addSubview: open_browser_btn];
}

#[cfg(target_os = "macos")]
unsafe fn make_button(frame: cocoa::foundation::NSRect, title: &str, selector: Sel, target: id) -> id {
    let btn: id = msg_send![class!(NSButton), alloc];
    let btn: id = msg_send![btn, initWithFrame: frame];
    let title_ns = NSString::alloc(nil).init_str(title);
    let _: () = msg_send![btn, setTitle: title_ns];
    let _: () = msg_send![btn, setBezelStyle: NSBezelStyle::NSRoundedBezelStyle as u64];
    let _: () = msg_send![btn, setTarget: target];
    let _: () = msg_send![btn, setAction: selector];
    btn
}

#[cfg(target_os = "macos")]
unsafe fn set_label_value(label: id, value: &str) {
    let ns_value = NSString::alloc(nil).init_str(value);
    let _: () = msg_send![label, setStringValue: ns_value];
}

#[cfg(target_os = "macos")]
unsafe fn create_action_target(action_tx: std::sync::mpsc::Sender<UiAction>) -> id {
    use objc::declare::ClassDecl;
    static mut CLASS: *const Class = std::ptr::null();

    if CLASS.is_null() {
        let mut decl = ClassDecl::new("LiRaysActionTarget", class!(NSObject)).expect("class decl");
        decl.add_ivar::<*mut c_void>("tx");
        decl.add_method(sel!(handleOpenConfig:), open_config as extern "C" fn(&Object, Sel, id));
        decl.add_method(sel!(handleRestart:), restart_service as extern "C" fn(&Object, Sel, id));
        decl.add_method(sel!(handleOpenBrowser:), open_browser as extern "C" fn(&Object, Sel, id));
        CLASS = decl.register();
    }

    let obj: id = msg_send![CLASS, alloc];
    let obj: id = msg_send![obj, init];
    let boxed = Box::new(action_tx);
    let ptr = Box::into_raw(boxed) as *mut c_void;
    (*obj).set_ivar("tx", ptr);
    obj
}

#[cfg(target_os = "macos")]
unsafe fn get_sender(this: &Object) -> std::sync::mpsc::Sender<UiAction> {
    let ptr: *mut c_void = *this.get_ivar("tx");
    let tx_ptr = ptr as *mut std::sync::mpsc::Sender<UiAction>;
    (*tx_ptr).clone()
}

#[cfg(target_os = "macos")]
extern "C" fn open_config(this: &Object, _cmd: Sel, _sender: id) {
    let tx = unsafe { get_sender(this) };
    let _ = tx.send(UiAction::OpenConfig);
}

#[cfg(target_os = "macos")]
extern "C" fn restart_service(this: &Object, _cmd: Sel, _sender: id) {
    let tx = unsafe { get_sender(this) };
    let _ = tx.send(UiAction::Restart);
}

#[cfg(target_os = "macos")]
extern "C" fn open_browser(this: &Object, _cmd: Sel, _sender: id) {
    let tx = unsafe { get_sender(this) };
    let _ = tx.send(UiAction::OpenBrowser);
}

#[cfg(target_os = "macos")]
fn handle_ui_action(action: UiAction, state: &mut UiState) {
    match action {
        UiAction::OpenConfig => {
            if let Some(cfg) = ensure_config_path(state) {
                if let Err(err) = open::that(&cfg) {
                    eprintln!("No se pudo abrir el editor: {err}");
                } else {
                    state.config_path = Some(cfg);
                }
            } else {
                eprintln!("No se encontró settings.yaml para abrir");
            }
        }
        UiAction::OpenBrowser => {
            if let Err(err) = webbrowser::open(&state.service_url) {
                eprintln!("No se pudo abrir el navegador: {err}");
            }
        }
        UiAction::Restart => {
            stop_server(&mut state.server_runtime);
            let new_config =
                load_runtime_config(state.user_config_arg.clone(), state.in_bundle);
            state.service_url =
                build_service_url(&new_config.host, new_config.port, new_config.tls_enabled);
            state.server_runtime = spawn_server(&new_config);
            wait_for_server_ready(&mut state.server_runtime);
            state.config_path = new_config.config_path.clone();
            #[cfg(target_os = "macos")]
            if let Some(label) = state.url_label {
                unsafe { set_label_value(label, &state.service_url) };
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn ensure_config_path(state: &mut UiState) -> Option<PathBuf> {
    // If user passed --config, ensure it exists (write default if missing)
    if let Some(user_path) = state.user_config_arg.clone() {
        if user_path.exists() {
            state.config_path = Some(user_path.clone());
            return Some(user_path);
        }
        if let Some(parent) = user_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if fs::write(&user_path, DEFAULT_SETTINGS_YAML).is_ok() {
            state.config_path = Some(user_path.clone());
            return Some(user_path);
        }
    }

    if let Some(path) = state.config_path.clone() {
        if path.exists() {
            return Some(path);
        }
    }

    if let Some(path) = resolve_default_config(state.in_bundle) {
        state.config_path = Some(path.clone());
        return Some(path);
    }

    None
}

#[cfg(target_os = "macos")]
fn stop_server(server_runtime: &mut ServerRuntime) {
    if !server_runtime.handle.is_finished() {
        server_runtime.handle.abort();
    }
}

#[cfg(target_os = "macos")]
struct UiState {
    user_config_arg: Option<PathBuf>,
    in_bundle: bool,
    service_url: String,
    server_runtime: ServerRuntime,
    url_label: Option<id>,
    config_path: Option<PathBuf>,
}

#[cfg(target_os = "macos")]
#[derive(Debug, Clone, Copy)]
enum UiAction {
    OpenConfig,
    Restart,
    OpenBrowser,
}

#[cfg(target_os = "macos")]
fn launch_window(
    user_config_arg: Option<PathBuf>,
    in_bundle: bool,
    service_url: String,
    config: RuntimeConfig,
    server_runtime: ServerRuntime,
) {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("LiRays SCADA")
        .with_inner_size(LogicalSize::new(520.0, 240.0))
        .with_min_inner_size(LogicalSize::new(480.0, 220.0))
        .build(&event_loop)
        .expect("failed to create window");

    let (action_tx, action_rx) = std::sync::mpsc::channel::<UiAction>();

    let mut state = UiState {
        user_config_arg,
        in_bundle,
        service_url,
        server_runtime,
        url_label: None,
        config_path: config.config_path.clone(),
    };

    unsafe {
        setup_cocoa_ui(&window, &mut state, action_tx);
    }

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        while let Ok(action) = action_rx.try_recv() {
            handle_ui_action(action, &mut state);
        }

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::LoopDestroyed => {
                stop_server(&mut state.server_runtime);
            }
            _ => {}
        }
    });
}
