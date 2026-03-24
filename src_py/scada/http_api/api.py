from contextlib import asynccontextmanager

from fastapi import FastAPI
from fastapi.staticfiles import StaticFiles
from starlette.responses import FileResponse

from scada.config import os

from .database.sqlite_engine import get_db_eng
from .routes.data import data_router, data_router_prefix
from .routes.mock import mock_router, mock_router_prefix

FRONTEND_BUILD = os.path.join(os.path.dirname(__file__), "static", "build")
INDEX_FILE = os.path.join(FRONTEND_BUILD, "index.html")


@asynccontextmanager
async def lifespan(app: FastAPI):
    await get_db_eng().initialize()
    yield


app = FastAPI(lifespan=lifespan)
app.include_router(mock_router, prefix=f"/api/{mock_router_prefix}")
app.include_router(data_router, prefix=f"/api/{data_router_prefix}")


if os.path.isfile(INDEX_FILE):
    app.mount("/", StaticFiles(directory=FRONTEND_BUILD, html=True), name="frontend")
