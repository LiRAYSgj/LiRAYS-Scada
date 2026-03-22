from typing import List, Optional

from pydantic import Field

from .base import Base


class TypedFloat(Base):
    FloatValue: float = Field(description="Float value to be set", examples=[23.12])


class TypedInteger(Base):
    IntegerValue: int = Field(description="Integer value to be set", examples=[23])


class TypedText(Base):
    TextValue: str = Field(description="Text value to be set", examples=["Some text"])


class TypedBoolean(Base):
    BooleanValue: bool = Field(
        description="Boolean value to be set", examples=[True, False]
    )


class Value(Base):
    typed: TypedFloat | TypedInteger | TypedText | TypedBoolean


class GetCmdPayload(Base):
    cmd_id: str = Field(
        ...,
        description="Unique identifier for the get command",
        examples=["cmd_get_12345"],
    )
    var_ids: List[str] = Field(
        ...,
        description="List of variable IDs to get values for",
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
    command_type: GetCommand


class GetResponseModel(Base):
    status: int = Field(
        ...,
        description="Operation status (0=Invalid, 1=OK, 2=Error)",
        examples=[1, 2],
        ge=0,
        le=2,
    )
    error_msg: Optional[str] = Field(
        None,
        description="Error message if operation failed",
        examples=[None, "Operation failed due to invalid input"],
    )
    response_type: GetResponse
