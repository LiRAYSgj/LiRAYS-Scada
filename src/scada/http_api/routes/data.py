import websockets
from fastapi import APIRouter, HTTPException, status

from ..model.add_cmd import AddCommandModel, AddResponseModel
from ..model.del_cmd import DelCommandModel, DelResponseModel
from ..model.get_cmd import GetCommandModel, GetResponseModel
from ..model.list_cmd import ListCommandModel, ListResponseModel
from ..model.set_cmd import SetCommandModel, SetResponseModel

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


@data_router.post(
    "/set",
    response_model=SetResponseModel,
    status_code=status.HTTP_200_OK,
    summary="Set values for variables.",
    description="Set values for specified variables.",
)
async def set_values(cmd_payload: SetCommandModel):
    return await cmd_executor(cmd_payload.model_dump_json(), SetResponseModel)


@data_router.post(
    "/get",
    response_model=GetResponseModel,
    status_code=status.HTTP_200_OK,
    summary="Get values for variables.",
    description="Get values for specified variables.",
)
async def get_values(cmd_payload: GetCommandModel):
    return await cmd_executor(cmd_payload.model_dump_json(), GetResponseModel)


@data_router.post(
    "/del",
    response_model=DelResponseModel,
    status_code=status.HTTP_200_OK,
    summary="Delete items.",
    description="Delete specified folders and variables.",
)
async def delete_items(cmd_payload: DelCommandModel):
    return await cmd_executor(cmd_payload.model_dump_json(), DelResponseModel)
