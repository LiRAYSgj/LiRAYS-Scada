from contextlib import asynccontextmanager
from typing import AsyncGenerator

from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine
from sqlmodel import SQLModel, select

from ...config import STATIC_DB
from ..model.mock import MockResource


class SQLiteEng:
    def __init__(self):
        self.db_url = f"sqlite+aiosqlite:///{STATIC_DB}"
        self.engine = create_async_engine(
            self.db_url, connect_args={"check_same_thread": False}, echo=False
        )

    @asynccontextmanager
    async def get_session(self) -> AsyncGenerator[AsyncSession, None]:
        async_session = async_sessionmaker(self.engine, class_=AsyncSession)
        async with async_session() as session:
            yield session

    async def initialize(self):
        async with self.engine.begin() as conn:
            await conn.run_sync(SQLModel.metadata.create_all)

    async def create_resource(self, resource: MockResource) -> MockResource:
        async with self.get_session() as session:
            session.add(resource)
            await session.commit()
            await session.refresh(resource)
            return resource

    async def list_resources(self) -> list[MockResource]:
        async with self.get_session() as session:
            result = await session.execute(select(MockResource))
            return list(result.scalars().all())


db_instance = SQLiteEng()


def get_db_eng() -> SQLiteEng:
    return db_instance
