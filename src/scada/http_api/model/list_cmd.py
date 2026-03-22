from typing import List, Optional

from pydantic import Field

from .base import Base


class ListCmdPayload(Base):
    cmd_id: str = Field(
        ..., description="Unique identifier for the command", examples=["cmd_12345"]
    )
    folder_id: Optional[str] = Field(
        None,
        description="ID of the folder to list items from. Pass null to list the root",
        examples=[None, "/folder/path"],
    )


class ListCommand(Base):
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
        ..., description="Unique identifier for the command", examples=["cmd_12345"]
    )
    folders: List[FolderResp] = Field(
        ..., description="List of folders in the response"
    )
    variables: List[VarResp] = Field(
        ..., description="List of variables in the response"
    )


class ListResponse(Base):
    List: ListRespPayload


class ListCommandModel(Base):
    command_type: ListCommand


class ListResponseModel(Base):
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
    response_type: ListResponse
