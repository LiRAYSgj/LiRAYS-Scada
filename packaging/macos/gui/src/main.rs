use std::process::Command;
use std::time::Duration;

use image::ImageFormat;
use serde::Serialize;
use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::{Icon, WindowBuilder};
use wry::WebViewBuilder;

const SERVICE_LABEL: &str = "com.lirays.scada";
const DASHBOARD_URL: &str = "http://127.0.0.1:8245/";
const SETTINGS_PATH: &str = "/Library/LiRAYS-Scada/settings.yaml";
const LOG_DIR: &str = "/Library/Logs/LiRAYS-Scada";

#[derive(Serialize, Clone)]
struct Status {
    running: bool,
    message: String,
}

fn load_icon() -> Icon {
    // Use bundled 512x512 PNG (from frontend/static/android-chrome-512x512.png)
    // to set the window/taskbar icon. This does not require .icns.
    const ICON_BYTES: &[u8] = include_bytes!("../../resources/app_icon.png");
    let img = image::load_from_memory_with_format(ICON_BYTES, ImageFormat::Png)
        .expect("icon decode");
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Icon::from_rgba(rgba.into_raw(), w, h).expect("icon convert")
}

fn run_osascript(cmd: &str) -> std::io::Result<bool> {
    // Escape for AppleScript string literal
    let escaped = cmd.replace('\\', "\\\\").replace('\"', "\\\"");
    let status = Command::new("osascript")
        .arg("-e")
        .arg(format!(
            r#"do shell script "{}" with administrator privileges"#,
            escaped
        ))
        .status()?;
    Ok(status.success())
}

fn status() -> Status {
    let out = Command::new("launchctl")
        .args(["print", &format!("system/{}", SERVICE_LABEL)])
        .output();
    match out {
        Ok(o) if o.status.success() => Status {
            running: true,
            message: "Running".into(),
        },
        Ok(o) => Status {
            running: false,
            message: String::from_utf8_lossy(&o.stderr).to_string(),
        },
        Err(e) => Status {
            running: false,
            message: e.to_string(),
        },
    }
}

fn start_service() -> Status {
    match run_osascript("launchctl kickstart -k system/com.lirays.scada") {
        Ok(true) => status(),
        Ok(false) => Status {
            running: false,
            message: "Start failed".into(),
        },
        Err(e) => Status {
            running: false,
            message: e.to_string(),
        },
    }
}

fn stop_service() -> Status {
    match run_osascript("launchctl bootout system/com.lirays.scada") {
        Ok(true) => status(),
        Ok(false) => Status {
            running: false,
            message: "Stop failed".into(),
        },
        Err(e) => Status {
            running: false,
            message: e.to_string(),
        },
    }
}

fn open_browser() {
    let _ = Command::new("open").arg(DASHBOARD_URL).status();
}

fn open_settings() {
    let _ = Command::new("open")
        .args(["-a", "TextEdit", SETTINGS_PATH])
        .status();
}

fn open_logs() {
    let _ = Command::new("open").arg(LOG_DIR).status();
}

fn uninstall(keep_data: bool) -> Status {
    let data_cmd = if keep_data {
        String::from("echo \"Keeping data & logs\";")
    } else {
        String::from("rm -rf \"/Library/Application Support/LiRAYS-Scada\" \"/Library/Logs/LiRAYS-Scada\";")
    };

    let script = format!(
        r#"bash -lc 'set -e;
launchctl bootout system/com.lirays.scada 2>/dev/null || true;
launchctl disable system/com.lirays.scada 2>/dev/null || true;
rm -f "/Library/LaunchDaemons/com.lirays.scada.plist";
rm -f "/usr/local/bin/lirays-scada";
{data_cmd}
pkgutil --forget com.lirays.scada 2>/dev/null || true;
(sleep 2; rm -rf "/Applications/LiRAYS Scada.app") &
'"#
    );

    match run_osascript(&script) {
        Ok(true) => {
            // Exit so the running app doesn't block its own removal
            std::process::exit(0);
        }
        Ok(false) => Status {
            running: false,
            message: "Uninstall failed".into(),
        },
        Err(e) => Status {
            running: false,
            message: e.to_string(),
        },
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoopBuilder::<serde_json::Value>::with_user_event().build();
    let icon = load_icon();

    let window = WindowBuilder::new()
        .with_title("LiRAYS Scada")
        .with_window_icon(Some(icon))
        .build(&event_loop)?;

    let html = format!(
        r#"
<!doctype html>
<html>
<head>
  <meta charset="utf-8">
  <title>LiRAYS Scada</title>
  <style>
    body {{
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      margin: 24px;
      background: #0b1727;
      color: #e6edf5;
    }}
    h1 {{ margin-top: 0; }}
    .status {{ margin: 12px 0; padding: 12px; border-radius: 8px; background: #132238; }}
    button {{
      margin: 6px;
      padding: 10px 14px;
      border: none;
      border-radius: 8px;
      background: #3b82f6;
      color: white;
      font-size: 14px;
      cursor: pointer;
    }}
    button.secondary {{ background: #1f2937; color: #e6edf5; }}
    button.danger {{ background: #b91c1c; }}
    button:disabled {{ opacity: 0.6; cursor: not-allowed; }}
    .row {{ display: flex; flex-wrap: wrap; gap: 8px; }}
    .muted {{ color: #9ca3af; font-size: 12px; margin-top: 6px; }}
  </style>
</head>
<body>
  <h1>LiRAYS Scada</h1>
  <div class="status" id="status">Checking status…</div>
  <div class="row">
    <button onclick="send('start')">Start</button>
    <button onclick="send('stop')">Stop</button>
    <button onclick="send('restart')">Restart</button>
    <button class="secondary" onclick="send('open-browser')">Open Dashboard</button>
    <button class="secondary" onclick="send('open-settings')">Open Settings</button>
    <button class="secondary" onclick="send('open-logs')">Open Logs Folder</button>
    <button class="secondary" onclick="send('refresh')">Refresh</button>
    <button class="danger" onclick="confirmUninstall()">Uninstall…</button>
  </div>
  <div style="margin-top:10px;">
    <label><input type="checkbox" id="keepData" checked> Keep data & logs</label>
    <div class="muted">Uninstall stops the service, removes the daemon and binary. Uncheck to also delete config/data/logs.</div>
  </div>
  <script>
    function send(action) {{
      const payload = {{ action }};
      const keepBox = document.getElementById('keepData');
      if (keepBox) payload.keepData = keepBox.checked;
      window.ipc.postMessage(JSON.stringify(payload));
    }}
    function confirmUninstall() {{
      if (confirm('Uninstall LiRAYS Scada?')) {{
        send('uninstall');
      }}
    }}
    function updateStatus(s) {{
      const box = document.getElementById('status');
      box.textContent = (s.running ? 'Running' : 'Stopped') + (s.message ? (' — ' + s.message) : '');
      box.style.background = s.running ? '#0f3a1a' : '#3a0f0f';
    }}
  </script>
</body>
</html>
        "#
    );

    let proxy = event_loop.create_proxy();
    let webview = WebViewBuilder::new(&window)
        .with_html(html)
        .with_ipc_handler(move |msg: String| {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&msg) {
                proxy
                    .send_event(v)
                    .expect("send event");
            }
        })
        .build()?;

    // initial status
    webview.evaluate_script(&format!(
        "updateStatus({});",
        serde_json::to_string(&status()).unwrap()
    ))?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(
            std::time::Instant::now() + Duration::from_millis(500),
        );
        match event {
            Event::NewEvents(StartCause::Init) => {
                let _ = webview.evaluate_script(&format!(
                    "updateStatus({});",
                    serde_json::to_string(&status()).unwrap()
                ));
            }
            Event::UserEvent(payload) => {
                let action = payload
                    .get("action")
                    .and_then(|a| a.as_str())
                    .unwrap_or("");
                let keep = payload
                    .get("keepData")
                    .and_then(|b| b.as_bool())
                    .unwrap_or(true);
                let st = match action {
                    "start" => start_service(),
                    "stop" => stop_service(),
                    "restart" => start_service(),
                    "open-browser" => {
                        open_browser();
                        status()
                    }
                    "open-settings" => {
                        open_settings();
                        status()
                    }
                    "open-logs" => {
                        open_logs();
                        status()
                    }
                    "uninstall" => uninstall(keep),
                    "refresh" => status(),
                    _ => status(),
                };
                let _ = webview.evaluate_script(&format!(
                    "updateStatus({});",
                    serde_json::to_string(&st).unwrap()
                ));
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
    #[allow(unreachable_code)]
    Ok(())
}
