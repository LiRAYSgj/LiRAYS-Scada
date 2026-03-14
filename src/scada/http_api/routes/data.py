import websockets
from fastapi import APIRouter, Depends, HTTPException, status
from google.protobuf.json_format import MessageToDict, ParseDict

from ...proto.namespace.commands_pb2 import Command, Response
from ..database.sqlite_engine import SQLiteEng, get_db_eng
from ..model.data_commands import (
    AddCommandModel,
    AddResponseModel,
    DelCommandModel,
    DelResponseModel,
    GetCommandModel,
    GetResponseModel,
    ListCommandModel,
    ListResponseModel,
    SetCommandModel,
    SetResponseModel,
)

data_router = APIRouter(tags=["Data"])
data_router_prefix = "/data"

WS_SERVER_URL = "ws://localhost:1236"


async def cmd_executor(cmd_payload, expected_cmd: str):
    try:
        payload_dict = cmd_payload.model_dump()
        command = ParseDict({expected_cmd: payload_dict}, Command())
    except Exception as e:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail=f"Invalid payload format: {e}",
        )

    if command.WhichOneof("command_type") != expected_cmd:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail=f"Invalid command type. Only '{expected_cmd}' is allowed on this endpoint.",
        )

    try:
        async with websockets.connect(WS_SERVER_URL) as websocket:
            # Send the binary serialized protobuf command
            await websocket.send(command.SerializeToString())

            # Receive the binary response
            response_data = await websocket.recv()

            # Parse it back into a Response object
            response = Response()
            response.ParseFromString(response_data)

            # Return as a regular JSON dictionary (matching the proto schema)
            resp = MessageToDict(response, preserving_proto_field_name=True)
            if resp["status"] == "OPERATION_STATUS_OK":
                return resp[expected_cmd]
            else:
                raise Exception(resp.get("error_msg"))

    except Exception as e:
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"WebSocket communication error: {e}",
        )


@data_router.post(
    "/list",
    response_model=ListResponseModel,
    status_code=status.HTTP_200_OK,
    summary="List a data directory.",
    description="List a data directory.",
)
async def list_command(cmd_payload: ListCommandModel):
    return await cmd_executor(cmd_payload, "list")


@data_router.post(
    "/add",
    response_model=AddResponseModel,
    status_code=status.HTTP_200_OK,
    summary="Add variables and directories.",
    description="Add variables and directories.",
)
async def add_command(cmd_payload: AddCommandModel):
    return await cmd_executor(cmd_payload, "add")


@data_router.post(
    "/set",
    response_model=SetResponseModel,
    status_code=status.HTTP_200_OK,
    summary="Set variable values.",
    description="Set variable values.",
)
async def set_command(cmd_payload: SetCommandModel):
    return await cmd_executor(cmd_payload, "set")


@data_router.post(
    "/get",
    response_model=GetResponseModel,
    status_code=status.HTTP_200_OK,
    summary="Get variable values.",
    description="Get variable values.",
)
async def get_command(cmd_payload: GetCommandModel):
    return await cmd_executor(cmd_payload, "get")


@data_router.delete(
    "/del",
    response_model=DelResponseModel,
    status_code=status.HTTP_200_OK,
    summary="Get variable values.",
    description="Get variable values.",
)
async def del_command(cmd_payload: DelCommandModel):
    return await cmd_executor(cmd_payload, "del")
