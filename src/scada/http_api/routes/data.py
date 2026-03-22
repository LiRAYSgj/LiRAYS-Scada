import websockets
from fastapi import APIRouter, HTTPException, status

from ..model.add_cmd import AddCommandModel, AddResponseModel
from ..model.list_cmd import ListCommandModel, ListResponseModel

data_router = APIRouter(tags=["Data"])
data_router_prefix = "/data"

WS_SERVER_URL = "ws://localhost:1236"


async def cmd_executor(command: str, resp_model):
    try:
        async with websockets.connect(WS_SERVER_URL) as web_sock:
            await web_sock.send(command)
            response = resp_model.model_validate_json(await web_sock.recv())
    except Exception as e:
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"WebSocket communication error: {e}",
        )
    if response.status != 1:
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Command returned an error: {response.error_msg}",
        )
    else:
        return response


@data_router.post(
    "/add",
    response_model=AddResponseModel,
    status_code=status.HTTP_200_OK,
    summary="Add new folders and variables.",
    description="Add new folders and variables to a specified data directory.",
)
async def add_items(cmd_payload: AddCommandModel):
    return await cmd_executor(cmd_payload.model_dump_json(), AddResponseModel)


@data_router.post(
    "/list",
    response_model=ListResponseModel,
    status_code=status.HTTP_200_OK,
    summary="List children of a data directory.",
    description="List children (folders and variables) of a specified data directory.",
)
async def list_items(cmd_payload: ListCommandModel):
    return await cmd_executor(cmd_payload.model_dump_json(), ListResponseModel)
