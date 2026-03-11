import http.server
import logging
import os
import socketserver
import threading
import time

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


def run_http_server(port=8080):
    frontend_dir = os.path.join(
        os.path.dirname(os.path.abspath(__file__)), "frontend", "dist"
    )
    if not os.path.exists(frontend_dir):
        logging.warning(
            f"Frontend dist directory not found: {frontend_dir}. "
            f"Make sure to run 'npm run build' inside the frontend directory."
        )
        # Fall back to serving the current directory so the server doesn't crash
        directory = os.path.dirname(os.path.abspath(__file__))
    else:
        directory = frontend_dir

    class Handler(http.server.SimpleHTTPRequestHandler):
        def __init__(self, *args, **kwargs):
            super().__init__(*args, directory=directory, **kwargs)

    # Use a subclass of TCPServer that allows address reuse
    class ReusableTCPServer(socketserver.TCPServer):
        allow_reuse_address = True

    with ReusableTCPServer(("", port), Handler) as httpd:
        logging.info(f"Serving Svelte frontend at http://localhost:{port}")
        httpd.serve_forever()


if __name__ == "__main__":
    http_thread = threading.Thread(target=run_http_server, daemon=True)
    http_thread.start()
    api_thread = ApiServer(app, "0.0.0.0", 1237)
    api_thread.start()

    serve("0.0.0.0", 1236, rt_dir)
    while True:
        time.sleep(1)
