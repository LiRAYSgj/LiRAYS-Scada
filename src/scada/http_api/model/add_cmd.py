from pydantic import Field

from .base import Base


class ItemMeta(Base):
    name: str = Field(
        ...,
        description="Name of the item to be added. This name will be used in MCP agent operations and should be descriptive for agent understanding.",
        examples=["MyFolder", "TemperatureSensor"],
    )
    i_type: int = Field(
        ...,
        description="Type of item to add (0=Invalid, 1=Folder, 2=Variable). Used for MCP protocol translation to determine operation type.",
        examples=[1, 2],
        ge=0,
        le=2,
    )
    var_d_type: int | None = Field(
        None,
        description="Data type of the variable (0=Invalid, 1=Integer, 2=Float, 3=Text, 4=Boolean). Only required for variables. This field is crucial for MCP agents to understand data handling requirements.",
        examples=[1, 2, 3, 4],
        ge=0,
        le=4,
    )


class AddCmdPayload(Base):
    cmd_id: str = Field(
        ...,
        description="Unique identifier for the add command. Required for MCP protocol tracking and response correlation.",
        examples=["cmd_add_12345"],
    )
    parent_id: str | None = Field(
        ...,
        description="ID of the parent folder where items will be added. Pass null to add to root. This field is essential for MCP agents to understand the target location for operations.",
        examples=[None, "/folder/path"],
    )
    items_meta: list[ItemMeta] = Field(
        ...,
        description="List of item metadata to be added. Each item in this list represents an operation that MCP agents can understand and process.",
        examples=[
            [
                ItemMeta(name="Folder1", i_type=1, var_d_type=None),
                ItemMeta(name="IntegerVar", i_type=2, var_d_type=1),
                ItemMeta(name="FloatVar", i_type=2, var_d_type=2),
                ItemMeta(name="TextVar", i_type=2, var_d_type=3),
                ItemMeta(name="BooleanVar", i_type=2, var_d_type=4),
            ]
        ],
    )


class AddCommand(Base):
    """
    Add command structure for MCP protocol compatibility.
    This command type can be translated between HTTP and WebSocket protocols
    for seamless integration with MCP-compatible AI agents.
    """

    Add: AddCmdPayload


class AddRespPayload(Base):
    cmd_id: str = Field(
        ...,
        description="Unique identifier for the add command",
        examples=["cmd_add_12345"],
    )


class AddResponse(Base):
    Add: AddRespPayload


class AddCommandModel(Base):
    """
    Add command model for MCP protocol compatibility.
    This model structure supports translation between HTTP and WebSocket protocols
    for seamless integration with MCP-compatible AI agents.
    """

    command_type: AddCommand


class AddResponseModel(Base):
    """
    Add response model for MCP protocol compatibility.
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
    error_msg: str | None = Field(
        None,
        description="Error message if operation failed. MCP agents can use this for error handling and reporting.",
        examples=[None, "Operation failed due to invalid input"],
    )
    response_type: AddResponse
