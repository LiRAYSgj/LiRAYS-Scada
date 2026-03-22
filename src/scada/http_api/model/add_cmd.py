from pydantic import Field

from .base import Base


class ItemMeta(Base):
    name: str = Field(
        ...,
        description="Name of the item to be added",
        examples=["MyFolder", "TemperatureSensor"],
    )
    i_type: int = Field(
        ...,
        description="Type of item to add (0=Invalid, 1=Folder, 2=Variable)",
        examples=[1, 2],
        ge=0,
        le=2,
    )
    var_d_type: int | None = Field(
        None,
        description="Data type of the variable (0=Invalid, 1=Integer, 2=Float, 3=Text, 4=Boolean). Only required for variables",
        examples=[1, 2, 3, 4],
        ge=0,
        le=4,
    )


class AddCmdPayload(Base):
    cmd_id: str = Field(
        ...,
        description="Unique identifier for the add command",
        examples=["cmd_add_12345"],
    )
    parent_id: str | None = Field(
        ...,
        description="ID of the parent folder where items will be added. Pass null to add to root",
        examples=[None, "/folder/path"],
    )
    items_meta: list[ItemMeta] = Field(
        ...,
        description="List of item metadata to be added",
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
    command_type: AddCommand


class AddResponseModel(Base):
    status: int = Field(
        ...,
        description="Operation status (0=Invalid, 1=OK, 2=Error)",
        examples=[1, 2],
        ge=0,
        le=2,
    )
    error_msg: str | None = Field(
        None,
        description="Error message if operation failed",
        examples=[None, "Operation failed due to invalid input"],
    )
    response_type: AddResponse
