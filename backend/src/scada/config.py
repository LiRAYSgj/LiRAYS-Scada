import os

SNAP_COMMON = os.environ.get("SNAP_COMMON", "/etc")

BIND_HOST = os.environ.get("BIND_HOST", "0.0.0.0")
BIND_RT_PORT = int(os.environ.get("BIND_SERVER_PORT", 8245))
BIND_HTTP_PORT = int(os.environ.get("BIND_HTTP_PORT", 8246))
DATA_DIR = os.environ.get("DATA_DIR", os.path.join(SNAP_COMMON, "liraysdata"))
RT_DATA_DIR = os.path.join(DATA_DIR, "rt_data")
STATIC_DB = os.path.join(DATA_DIR, "static.db")


def initialize_conf():
    os.makedirs(DATA_DIR, exist_ok=True)
    os.makedirs(RT_DATA_DIR, exist_ok=True)
