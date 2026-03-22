from typing import List, Optional

from pydantic import Field

from .base import Base


class DelCmdPayload(Base):
    cmd_id: str = Field(
        ...,
        description="Unique identifier for the delete command",
        examples=["cmd_del_12345"],
    )
    item_ids: List[str] = Field(
        ...,
        description="List of item IDs to delete",
        examples=[["item_folder1", "item_var1"]],
    )


class DelCommand(Base):
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
    command_type: DelCommand


class DelResponseModel(Base):
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
    response_type: DelResponse
