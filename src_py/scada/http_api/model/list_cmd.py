from typing import List, Optional

from pydantic import Field

from .base import Base


class ListCmdPayload(Base):
    cmd_id: str = Field(
        ...,
        description="Unique identifier for the command. Required for MCP protocol tracking and response correlation.",
        examples=["cmd_12345"],
    )
    folder_id: Optional[str] = Field(
        None,
        description="ID of the folder to list items from. Pass null to list the root. This field is essential for MCP agents to understand the target location for operations.",
        examples=[None, "/folder/path"],
    )


class ListCommand(Base):
    """
    List command structure for MCP protocol compatibility.
    This command type can be translated between HTTP and WebSocket protocols
    for seamless integration with MCP-compatible AI agents.
    """

    List: ListCmdPayload


class FolderResp(Base):
    id: str = Field(
        ...,
        description="Unique identifier for the folder",
        examples=["/folder/path/MyFolder"],
    )
    name: str = Field(..., description="Name of the folder", examples=["MyFolder"])


class VarResp(Base):
    id: str = Field(
        ...,
        description="Unique identifier for the variable",
        examples=["/folder/path/TemperatureSensor"],
    )
    name: str = Field(
        ..., description="Name of the variable", examples=["TemperatureSensor"]
    )
    var_d_type: int = Field(
        ...,
        description="Variable data type (0=Invalid, 1=Integer, 2=Float, 3=Text, 4=Boolean)",
        examples=[1, 2, 3, 4],
        ge=0,
        le=4,
    )


class ListRespPayload(Base):
    cmd_id: str = Field(
        ...,
        description="Unique identifier for the command. Required for MCP protocol tracking and response correlation.",
        examples=["cmd_12345"],
    )
    folders: List[FolderResp] = Field(
        ...,
        description="List of folders in the response. MCP agents can use this to understand the directory structure.",
    )
    variables: List[VarResp] = Field(
        ...,
        description="List of variables in the response. MCP agents can use this to understand available data points.",
    )


class ListResponse(Base):
    List: ListRespPayload


class ListCommandModel(Base):
    """
    List command model for MCP protocol compatibility.
    This model structure supports translation between HTTP and WebSocket protocols
    for seamless integration with MCP-compatible AI agents.
    """

    command_type: ListCommand


class ListResponseModel(Base):
    """
    List response model for MCP protocol compatibility.
    This model structure supports translation between HTTP and WebSocket protocols
    for seamless integration with MCP-compatible AI agents.
    """

    status: int = Field(
        ...,
        description="Operation status (0=Invalid, 1=OK, 2=Error). For MCP compatibility, status 1 indicates successful command execution that can be reported back to agents.",
        examples=[1, 2],
        ge=0,
        le=2,
    )
    error_msg: Optional[str] = Field(
        None,
        description="Error message if operation failed",
        examples=[None, "Operation failed due to invalid input"],
    )
    response_type: ListResponse
