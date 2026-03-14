from contextlib import asynccontextmanager
from threading import Thread

import uvicorn
from fastapi import FastAPI

from .database.sqlite_engine import get_db_eng
from .routes.data import data_router, data_router_prefix
from .routes.mock import mock_router, mock_router_prefix


@asynccontextmanager
async def lifespan(app: FastAPI):
    await get_db_eng().initialize()
    yield


app = FastAPI(lifespan=lifespan)
app.include_router(mock_router, prefix=mock_router_prefix)
app.include_router(data_router, prefix=data_router_prefix)


class ApiServer(Thread):
    def __init__(self, app, host: str, port: int):
        Thread.__init__(self, daemon=True)
        config = uvicorn.Config(
            app, host=host, port=port, log_level="info", loop="asyncio"
        )
        self.server = uvicorn.Server(config)

    def run(self):
        self.server.run()

    def stop(self):
        self.server.should_exit = True
