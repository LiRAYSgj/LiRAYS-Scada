from typing import List, Optional

from pydantic import Field

from .base import Base


class TypedFloat(Base):
    FloatValue: float = Field(
        description="Float value to be set. Used in MCP operations for data retrieval and reporting.",
        examples=[23.12],
    )


class TypedInteger(Base):
    IntegerValue: int = Field(
        description="Integer value to be set. Used in MCP operations for data retrieval and reporting.",
        examples=[23],
    )


class TypedText(Base):
    TextValue: str = Field(
        description="Text value to be set. Used in MCP operations for data retrieval and reporting.",
        examples=["Some text"],
    )


class TypedBoolean(Base):
    BooleanValue: bool = Field(
        description="Boolean value to be set. Used in MCP operations for data retrieval and reporting.",
        examples=[True, False],
    )


class Value(Base):
    typed: TypedFloat | TypedInteger | TypedText | TypedBoolean


class GetCmdPayload(Base):
    cmd_id: str = Field(
        ...,
        description="Unique identifier for the get command. Required for MCP protocol tracking and response correlation.",
        examples=["cmd_get_12345"],
    )
    var_ids: List[str] = Field(
        ...,
        description="List of variable IDs to get values for. Each ID should be a valid path that MCP agents can understand and process.",
        examples=[
            [
                "/devices/sensors/NewFolder/NewFloatVariable",
                "/devices/sensors/NewFolder/NewTextVariable",
                "/devices/sensors/NewFolder/NewIntegerVariable",
                "/devices/sensors/NewFolder/NewBooleanVariable",
            ]
        ],
    )


class GetCommand(Base):
    """
    Get command structure for MCP protocol compatibility.
    This command type can be translated between HTTP and WebSocket protocols
    for seamless integration with MCP-compatible AI agents.
    """

    Get: GetCmdPayload


class TypedValue(Base):
    value: Value | None = Field(
        None,
        description="Value of the variable (contains one of: integer_value, float_value, text_value, boolean_value). Null if variable not found",
        examples=[
            Value(typed=TypedFloat(FloatValue=23.12)),
            Value(typed=TypedInteger(IntegerValue=34)),
            Value(typed=TypedText(TextValue="Some text")),
            Value(typed=TypedBoolean(BooleanValue=True)),
            None,
        ],
    )


class GetRespPayload(Base):
    cmd_id: str = Field(
        ...,
        description="Unique identifier for the get command",
        examples=["cmd_get_12345"],
    )
    var_values: List[TypedValue] = Field(
        ...,
        description="List of variable values",
    )


class GetResponse(Base):
    Get: GetRespPayload


class GetCommandModel(Base):
    """
    Get command model for MCP protocol compatibility.
    This model structure supports translation between HTTP and WebSocket protocols
    for seamless integration with MCP-compatible AI agents.
    """

    command_type: GetCommand


class GetResponseModel(Base):
    """
    Get response model for MCP protocol compatibility.
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
    response_type: GetResponse
