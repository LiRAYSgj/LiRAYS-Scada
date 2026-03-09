import logging
import time

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

if __name__ == "__main__":
    serve(
        "0.0.0.0",
        1236,
        "/Users/alejandro/Local/LiRAYS/LiRAYS-Scada/demo_data"
    )
    logging.debug("Hello!")
    while True:
        time.sleep(1)
