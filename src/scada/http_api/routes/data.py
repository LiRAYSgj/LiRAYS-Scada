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
    """
    Execute a command via WebSocket and return the response.

    This function serves as the core executor for all MCP-compatible API commands,
    translating HTTP requests into WebSocket protocol operations for the backend.

    Args:
        command: JSON string of the command to execute
        resp_model: Pydantic model for response validation

    Returns:
        Parsed response from the WebSocket backend

    Raises:
        HTTPException: When WebSocket communication fails or command returns error
    """
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
    description="Add new folders and variables to a specified data directory. This endpoint supports MCP protocol translation for AI agents to create new items in the data structure.",
)
async def add_items(cmd_payload: AddCommandModel):
    """
    Add new folders and variables to the data directory.

    This endpoint is MCP-compatible and translates HTTP requests into WebSocket
    operations for the backend. It supports AI agents using MCP protocol to
    create new items in the data structure.

    Args:
        cmd_payload: AddCommandModel containing the command details

    Returns:
        AddResponseModel with the result of the add operation

    Example:
        {
            "command_type": {
                "Add": {
                    "cmd_id": "cmd_add_12345",
                    "parent_id": "/root/folder",
                    "items_meta": [
                        {
                            "name": "NewFolder",
                            "i_type": 1,
                            "var_d_type": null
                        }
                    ]
                }
            },
            "status": 1,
            "error_msg": null,
            "response_type": {
                "Add": {
                    "cmd_id": "cmd_add_12345"
                }
            }
        }
    """
    return await cmd_executor(cmd_payload.model_dump_json(), AddResponseModel)


@data_router.post(
    "/list",
    response_model=ListResponseModel,
    status_code=status.HTTP_200_OK,
    summary="List children of a data directory.",
    description="List children (folders and variables) of a specified data directory. This endpoint supports MCP protocol translation for AI agents to explore the data structure.",
)
async def list_items(cmd_payload: ListCommandModel):
    """
    List children (folders and variables) of a specified data directory.

    This endpoint is MCP-compatible and translates HTTP requests into WebSocket
    operations for the backend. It supports AI agents using MCP protocol to
    explore and navigate the data structure.

    Args:
        cmd_payload: ListCommandModel containing the command details

    Returns:
        ListResponseModel with the result of the list operation

    Example:
        {
            "command_type": {
                "List": {
                    "cmd_id": "cmd_12345",
                    "folder_id": "/root/folder"
                }
            },
            "status": 1,
            "error_msg": null,
            "response_type": {
                "List": {
                    "cmd_id": "cmd_12345",
                    "folders": [],
                    "variables": []
                }
            }
        }
    """
    return await cmd_executor(cmd_payload.model_dump_json(), ListResponseModel)


@data_router.post(
    "/set",
    response_model=SetResponseModel,
    status_code=status.HTTP_200_OK,
    summary="Set values for variables.",
    description="Set values for specified variables. This endpoint supports MCP protocol translation for AI agents to update data values.",
)
async def set_values(cmd_payload: SetCommandModel):
    """
    Set values for specified variables.

    This endpoint is MCP-compatible and translates HTTP requests into WebSocket
    operations for the backend. It supports AI agents using MCP protocol to
    update variable values in the data structure.

    Args:
        cmd_payload: SetCommandModel containing the command details

    Returns:
        SetResponseModel with the result of the set operation

    Example:
        {
            "command_type": {
                "Set": {
                    "cmd_id": "cmd_set_12345",
                    "var_ids_values": [
                        {
                            "var_id": "/root/temperature",
                            "value": {
                                "typed": {
                                    "FloatValue": 23.5
                                }
                            }
                        }
                    ]
                }
            },
            "status": 1,
            "error_msg": null,
            "response_type": {
                "Set": {
                    "cmd_id": "cmd_set_12345"
                }
            }
        }
    """
    return await cmd_executor(cmd_payload.model_dump_json(), SetResponseModel)


@data_router.post(
    "/get",
    response_model=GetResponseModel,
    status_code=status.HTTP_200_OK,
    summary="Get values for variables.",
    description="Get values for specified variables. This endpoint supports MCP protocol translation for AI agents to retrieve data values.",
)
async def get_values(cmd_payload: GetCommandModel):
    """
    Get values for specified variables.

    This endpoint is MCP-compatible and translates HTTP requests into WebSocket
    operations for the backend. It supports AI agents using MCP protocol to
    retrieve variable values from the data structure.

    Args:
        cmd_payload: GetCommandModel containing the command details

    Returns:
        GetResponseModel with the result of the get operation

    Example:
        {
            "command_type": {
                "Get": {
                    "cmd_id": "cmd_get_12345",
                    "var_ids": ["/root/temperature"]
                }
            },
            "status": 1,
            "error_msg": null,
            "response_type": {
                "Get": {
                    "cmd_id": "cmd_get_12345",
                    "var_values": [
                        {
                            "value": {
                                "typed": {
                                    "FloatValue": 23.5
                                }
                            }
                        }
                    ]
                }
            }
        }
    """
    return await cmd_executor(cmd_payload.model_dump_json(), GetResponseModel)


@data_router.post(
    "/del",
    response_model=DelResponseModel,
    status_code=status.HTTP_200_OK,
    summary="Delete items.",
    description="Delete specified folders and variables. This endpoint supports MCP protocol translation for AI agents to remove items from the data structure.",
)
async def delete_items(cmd_payload: DelCommandModel):
    """
    Delete specified folders and variables.

    This endpoint is MCP-compatible and translates HTTP requests into WebSocket
    operations for the backend. It supports AI agents using MCP protocol to
    remove items from the data structure.

    Args:
        cmd_payload: DelCommandModel containing the command details

    Returns:
        DelResponseModel with the result of the delete operation

    Example:
        {
            "command_type": {
                "Del": {
                    "cmd_id": "cmd_del_12345",
                    "item_ids": ["/root/folder_to_delete"]
                }
            },
            "status": 1,
            "error_msg": null,
            "response_type": {
                "Del": {
                    "cmd_id": "cmd_del_12345"
                }
            }
        }
    """
    return await cmd_executor(cmd_payload.model_dump_json(), DelResponseModel)
