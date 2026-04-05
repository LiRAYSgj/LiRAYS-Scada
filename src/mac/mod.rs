use std::{
    env,
    fs,
    os::raw::c_void,
    path::PathBuf,
    sync::mpsc,
    thread,
    time::Duration,
};

use cocoa::{
    appkit::{NSApp, NSBezelStyle},
    base::{id, nil},
    foundation::{NSString, NSPoint, NSRect, NSSize},
};
use objc::{
    class, msg_send,
    runtime::{Class, Object, Sel},
    sel, sel_impl,
};
use open;
use tao::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::macos::WindowExtMacOS,
    window::WindowBuilder,
};
use webbrowser;

use crate::{
    build_service_url, load_runtime_config, spawn_server, RuntimeConfig, ServerRuntime,
    DEFAULT_SETTINGS_YAML,
};

#[derive(Debug, Clone, Copy)]
enum UiAction {
    OpenConfig,
    Restart,
    OpenBrowser,
}

struct UiState {
    user_config_arg: Option<PathBuf>,
    in_bundle: bool,
    service_url: String,
    server_runtime: ServerRuntime,
    url_label: Option<id>,
    config_path: Option<PathBuf>,
    default_yaml: &'static str,
}

pub(crate) fn running_in_app_bundle() -> bool {
    env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().and_then(|p| p.parent()).and_then(|p| p.parent()).map(PathBuf::from))
        .and_then(|bundle| bundle.extension().map(|ext| ext == "app"))
        .unwrap_or(false)
}

pub(crate) fn mac_default_data_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join("Library/Application Support/LiRays-Scada"))
}

fn bundle_resources_dir() -> Option<PathBuf> {
    env::current_exe().ok().and_then(|exe| {
        exe.parent()
            .and_then(|p| p.parent()) // Contents
            .map(|contents| contents.join("Resources"))
    })
}

pub(crate) fn resolve_default_config(in_bundle: bool, default_yaml: &str) -> Option<PathBuf> {
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

    if let Some(app_cfg) = app_support {
        if let Some(parent) = app_cfg.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if fs::write(&app_cfg, default_yaml).is_ok() {
            return Some(app_cfg);
        }
    }

    None
}

pub(crate) fn wait_for_server_ready(server_runtime: &mut ServerRuntime) {
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
                thread::sleep(Duration::from_millis(25));
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

pub(crate) fn launch_window(
    user_config_arg: Option<PathBuf>,
    in_bundle: bool,
    service_url: String,
    config: RuntimeConfig,
    server_runtime: ServerRuntime,
    default_yaml: &'static str,
) {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("LiRays SCADA")
        .with_inner_size(LogicalSize::new(520.0, 240.0))
        .with_min_inner_size(LogicalSize::new(480.0, 220.0))
        .build(&event_loop)
        .expect("failed to create window");

    let (action_tx, action_rx) = mpsc::channel::<UiAction>();

    let mut state = UiState {
        user_config_arg,
        in_bundle,
        service_url,
        server_runtime,
        url_label: None,
        config_path: config.config_path.clone(),
        default_yaml,
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

unsafe fn setup_cocoa_ui(
    window: &tao::window::Window,
    state: &mut UiState,
    action_tx: mpsc::Sender<UiAction>,
) {
    let ns_window: id = window.ns_window() as id;

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

unsafe fn make_button(frame: NSRect, title: &str, selector: Sel, target: id) -> id {
    let btn: id = msg_send![class!(NSButton), alloc];
    let btn: id = msg_send![btn, initWithFrame: frame];
    let title_ns = NSString::alloc(nil).init_str(title);
    let _: () = msg_send![btn, setTitle: title_ns];
    let _: () = msg_send![btn, setBezelStyle: NSBezelStyle::NSRoundedBezelStyle as u64];
    let _: () = msg_send![btn, setTarget: target];
    let _: () = msg_send![btn, setAction: selector];
    btn
}

unsafe fn set_label_value(label: id, value: &str) {
    let ns_value = NSString::alloc(nil).init_str(value);
    let _: () = msg_send![label, setStringValue: ns_value];
}

unsafe fn create_action_target(action_tx: mpsc::Sender<UiAction>) -> id {
    use objc::declare::ClassDecl;
    static mut CLASS: *const Class = std::ptr::null();

    if CLASS.is_null() {
        let mut decl = ClassDecl::new("LiRaysActionTarget", class!(NSObject)).expect("class decl");
        decl.add_ivar::<*mut c_void>("tx");
        decl.add_method(sel!(handleOpenConfig:), open_config as extern "C" fn(&Object, Sel, id));
        decl.add_method(sel!(handleRestart:), restart_service as extern "C" fn(&Object, Sel, id));
        decl.add_method(sel!(handleOpenBrowser:), open_browser as extern "C" fn(&Object, Sel, id));
        unsafe {
            CLASS = decl.register();
        }
    }

    let obj: id = msg_send![CLASS, alloc];
    let obj: id = msg_send![obj, init];
    let boxed = Box::new(action_tx);
    let ptr = Box::into_raw(boxed) as *mut c_void;
    unsafe {
        (*obj).set_ivar("tx", ptr);
    }
    obj
}

unsafe fn get_sender(this: &Object) -> mpsc::Sender<UiAction> {
    let ptr: *mut c_void = *this.get_ivar("tx");
    let tx_ptr = ptr as *mut mpsc::Sender<UiAction>;
    (*tx_ptr).clone()
}

extern "C" fn open_config(this: &Object, _cmd: Sel, _sender: id) {
    let tx = unsafe { get_sender(this) };
    let _ = tx.send(UiAction::OpenConfig);
}

extern "C" fn restart_service(this: &Object, _cmd: Sel, _sender: id) {
    let tx = unsafe { get_sender(this) };
    let _ = tx.send(UiAction::Restart);
}

extern "C" fn open_browser(this: &Object, _cmd: Sel, _sender: id) {
    let tx = unsafe { get_sender(this) };
    let _ = tx.send(UiAction::OpenBrowser);
}

fn handle_ui_action(action: UiAction, state: &mut UiState) {
    match action {
        UiAction::OpenConfig => {
            if let Some(cfg) = ensure_config_path(state, state.default_yaml) {
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
            if let Some(label) = state.url_label {
                unsafe { set_label_value(label, &state.service_url) };
            }
        }
    }
}

fn ensure_config_path(state: &mut UiState, default_yaml: &str) -> Option<PathBuf> {
    if let Some(user_path) = state.user_config_arg.clone() {
        if user_path.exists() {
            state.config_path = Some(user_path.clone());
            return Some(user_path);
        }
        if let Some(parent) = user_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if fs::write(&user_path, default_yaml).is_ok() {
            state.config_path = Some(user_path.clone());
            return Some(user_path);
        }
    }

    if let Some(path) = state.config_path.clone() {
        if path.exists() {
            return Some(path);
        }
    }

    if let Some(path) = resolve_default_config(state.in_bundle, DEFAULT_SETTINGS_YAML) {
        state.config_path = Some(path.clone());
        return Some(path);
    }

    None
}

fn stop_server(server_runtime: &mut ServerRuntime) {
    if !server_runtime.handle.is_finished() {
        server_runtime.handle.abort();
    }
}
