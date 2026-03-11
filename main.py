import logging
import os
import subprocess
import time
from pathlib import Path
from typing import Optional

from scada.data_dir import rt_dir
from scada.http_api.api import ApiServer, app
from scada.rustmod import serve


# Configure Python's logging to show DEBUG level
#
class ShortLevelFormatter(logging.Formatter):
    def format(self, record):
        record.levelname = record.levelname[:3]
        return super().format(record)


handler = logging.StreamHandler()
handler.setFormatter(
    ShortLevelFormatter(
        "%(asctime)s [%(levelname)s]: %(message)s",
        datefmt="%Y-%m-%d %H:%M:%S",
    )
)

logger = logging.getLogger()
logger.setLevel(logging.DEBUG)
logger.addHandler(handler)
logging.basicConfig()


REPO_ROOT = Path(__file__).resolve().parent
FRONTEND_DIR = Path(os.getenv("SCADA_FRONTEND_DIR", str(REPO_ROOT / "frontend"))).resolve()
DEMO_DATA_DIR = os.getenv("SCADA_DEMO_DATA_DIR", rt_dir)
RUST_HOST = os.getenv("SCADA_RUST_HOST", "0.0.0.0")
RUST_PORT = int(os.getenv("SCADA_RUST_PORT", "1236"))
FRONTEND_MODE = os.getenv("SCADA_FRONTEND_MODE", "production").strip().lower()
FRONTEND_PORT = os.getenv("SCADA_FRONTEND_PORT", "3000")
FRONTEND_HOST = os.getenv("SCADA_FRONTEND_HOST", "0.0.0.0")
FORCE_FRONTEND_BUILD = os.getenv("SCADA_FRONTEND_FORCE_BUILD", "").lower() in {"1", "true", "yes"}


def build_frontend_if_needed(frontend_dir: Path) -> None:
    build_dir = frontend_dir / "build"
    if build_dir.exists() and not FORCE_FRONTEND_BUILD:
        logger.info("Using existing Svelte build at %s", build_dir)
        return

    reason = "forced rebuild" if FORCE_FRONTEND_BUILD else "missing build directory"
    logger.info("Building Svelte frontend (%s)", reason)
    completed = subprocess.run(
        ["npm", "run", "build"],
        cwd=str(frontend_dir),
        text=True,
        capture_output=True,
        check=False,
    )
    if completed.stdout:
        logger.info(completed.stdout.strip())
    if completed.stderr:
        logger.warning(completed.stderr.strip())
    if completed.returncode != 0:
        raise RuntimeError("Failed to build frontend")


def run_svelte_server() -> subprocess.Popen[str]:
    frontend_dir = FRONTEND_DIR
    if not frontend_dir.exists():
        raise FileNotFoundError(f"Frontend directory not found: {frontend_dir}")

    if FRONTEND_MODE in {"dev", "development"}:
        command = ["npm", "run", "dev", "--", "--host", FRONTEND_HOST, "--port", FRONTEND_PORT]
        logger.info(
            "Starting Svelte frontend in dev mode at http://%s:%s",
            FRONTEND_HOST,
            FRONTEND_PORT,
        )
    else:
        build_frontend_if_needed(frontend_dir)
        command = ["npm", "run", "start"]
        logger.info("Starting Svelte frontend in production mode on port %s", FRONTEND_PORT)

    process_env = os.environ.copy()
    process_env["HOST"] = FRONTEND_HOST
    process_env["PORT"] = FRONTEND_PORT

    return subprocess.Popen(
        command,
        cwd=str(frontend_dir),
        env=process_env,
        text=True,
        start_new_session=True,
    )


def stop_process(process: Optional[subprocess.Popen[str]], name: str) -> None:
    if process is None:
        return
    if process.poll() is not None:
        logger.info("%s process already exited with code %s", name, process.returncode)
        return

    logger.info("Stopping %s process (pid=%s)", name, process.pid)
    try:
        os.killpg(process.pid, signal.SIGTERM)
        process.wait(timeout=10)
        logger.info("%s process stopped cleanly", name)
    except subprocess.TimeoutExpired:
        logger.warning("%s did not stop in time, forcing kill", name)
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        logger.info("%s process already terminated", name)


if __name__ == "__main__":
    frontend_process: Optional[subprocess.Popen[str]] = None
    api_thread: Optional[ApiServer] = None

    try:
        frontend_process = run_svelte_server()
        logger.info("Svelte process started with pid=%s", frontend_process.pid)

        api_thread = ApiServer(app, "0.0.0.0", 1237)
        api_thread.start()

        logger.info("Starting Rust backend at ws://%s:%s", RUST_HOST, RUST_PORT)
        serve(RUST_HOST, RUST_PORT, DEMO_DATA_DIR)
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        logger.info("Received KeyboardInterrupt, shutting down")
    finally:
        if api_thread:
            logger.info("Stopping HTTP API")
            api_thread.stop()
            api_thread.join(timeout=10)
        stop_process(frontend_process, "Svelte")
