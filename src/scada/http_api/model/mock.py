from sqlmodel import SQLModel, Field


class MockResource(SQLModel, table=True):
    id: int = Field(primary_key=True)
    name: str = Field(index=True)
    description: str = Field()
