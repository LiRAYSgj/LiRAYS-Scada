import logging
import signal
import sys

import uvicorn

from scada.config import (
    BIND_HOST,
    BIND_HTTP_PORT,
    BIND_RT_PORT,
    DATA_DIR,
    initialize_conf,
)
from scada.http_api.api import app
from scada.rustmod import serve


# Configure Python's logging to show DEBUG level
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
logger.setLevel(logging.INFO)
logger.addHandler(handler)
logging.basicConfig()


def signal_handler(signum, frame):
    logger.info("Received signal %s, shutting down gracefully...", signum)
    sys.exit(0)


def main():
    initialize_conf()

    # Set up signal handlers for graceful shutdown
    signal.signal(signal.SIGINT, signal_handler)
    signal.signal(signal.SIGTERM, signal_handler)

    try:
        logging.info("Starting RT service at ws://%s:%s", BIND_HOST, BIND_RT_PORT)
        serve(BIND_HOST, BIND_RT_PORT, DATA_DIR)

        logging.info("Starting HTTP service at http://%s:%s", BIND_HOST, BIND_HTTP_PORT)
        config = uvicorn.Config(
            app, host=BIND_HOST, port=BIND_HTTP_PORT, log_level="info", loop="asyncio"
        )
        server = uvicorn.Server(config)
        server.run()

    except KeyboardInterrupt:
        logger.info("Received KeyboardInterrupt, shutting down")
    except Exception as e:
        logger.error("Error occurred: %s", e)
    finally:
        logger.info("Shutdown complete")
