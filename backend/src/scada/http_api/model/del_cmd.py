from typing import List, Optional

from pydantic import Field

from .base import Base


class DelCmdPayload(Base):
    cmd_id: str = Field(
        ...,
        description="Unique identifier for the delete command. Required for MCP protocol tracking and response correlation.",
        examples=["cmd_del_12345"],
    )
    item_ids: List[str] = Field(
        ...,
        description="List of item IDs to delete. Each ID should be a valid path that MCP agents can understand and process.",
        examples=[["item_folder1", "item_var1"]],
    )


class DelCommand(Base):
    """
    Delete command structure for MCP protocol compatibility.
    This command type can be translated between HTTP and WebSocket protocols
    for seamless integration with MCP-compatible AI agents.
    """

    Del: DelCmdPayload


class DelRespPayload(Base):
    cmd_id: str = Field(
        ...,
        description="Unique identifier for the delete command",
        examples=["cmd_del_12345"],
    )


class DelResponse(Base):
    Del: DelRespPayload


class DelCommandModel(Base):
    """
    Delete command model for MCP protocol compatibility.
    This model structure supports translation between HTTP and WebSocket protocols
    for seamless integration with MCP-compatible AI agents.
    """

    command_type: DelCommand


class DelResponseModel(Base):
    """
    Delete response model for MCP protocol compatibility.
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
        description="Error message if operation failed. MCP agents can use this for error handling and reporting.",
        examples=[None, "Operation failed due to invalid input"],
    )
    response_type: DelResponse
