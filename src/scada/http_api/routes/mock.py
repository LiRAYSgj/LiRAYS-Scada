from fastapi import APIRouter, Depends, HTTPException, status

from ..model.mock import MockResource
from ..database.sqlite_engine import SQLiteEng, get_db_eng

mock_router = APIRouter(tags=["Mock Resource"])
mock_router_prefix = "/mock-resource"


@mock_router.post(
    "/resource/",
    response_model=MockResource
)
async def create_resource(
    payload: MockResource,
    db: SQLiteEng = Depends(get_db_eng)
):
    try:
        return await db.create_resource(payload)
    except Exception as err:
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Error: {err}.",
        )


@mock_router.get("/resource/", response_model=list[MockResource])
async def list_resources(
    db: SQLiteEng = Depends(get_db_eng)
):
    try:
        return await db.list_resources()
    except Exception as err:
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Error: {err}.",
        )
