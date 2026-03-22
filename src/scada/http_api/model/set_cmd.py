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


class VarIdValue(Base):
    var_id: str = Field(
        ...,
        description="ID of the variable to set",
        examples=["/folder/path/var_temp_sensor"],
    )
    value: Value = Field(
        ...,
        description="Value to set for the variable.",
    )


class SetCmdPayload(Base):
    cmd_id: str = Field(
        ...,
        description="Unique identifier for the set command",
        examples=["cmd_set_12345"],
    )
    var_ids_values: List[VarIdValue] = Field(
        ...,
        description="List of variable IDs and their values to set",
        examples=[
            [
                VarIdValue(
                    var_id="/folder/path/float_var",
                    value=Value(typed=TypedFloat(FloatValue=23.12)),
                ),
                VarIdValue(
                    var_id="/folder/path/integer_var",
                    value=Value(typed=TypedInteger(IntegerValue=34)),
                ),
                VarIdValue(
                    var_id="/folder/path/text_var",
                    value=Value(typed=TypedText(TextValue="Some text")),
                ),
                VarIdValue(
                    var_id="/folder/path/boolean_var",
                    value=Value(typed=TypedBoolean(BooleanValue=True)),
                ),
            ]
        ],
    )


class SetCommand(Base):
    Set: SetCmdPayload


class SetRespPayload(Base):
    cmd_id: str = Field(
        ...,
        description="Unique identifier for the set command",
        examples=["cmd_set_12345"],
    )


class SetResponse(Base):
    Set: SetRespPayload


class SetCommandModel(Base):
    command_type: SetCommand


class SetResponseModel(Base):
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
    response_type: SetResponse
