from typing import Any, Literal

from pydantic import BaseModel, Field


class FolderMeta(BaseModel):
    name: str = Field()
    i_type: Literal["ITEM_TYPE_FOLDER"] = Field("ITEM_TYPE_FOLDER")


class VarMeta(BaseModel):
    name: str = Field()
    i_type: Literal["ITEM_TYPE_VARIABLE"] = Field("ITEM_TYPE_VARIABLE")
    var_d_type: Literal[
        "VAR_DATA_TYPE_INTEGER",
        "VAR_DATA_TYPE_FLOAT",
        "VAR_DATA_TYPE_TEXT",
        "VAR_DATA_TYPE_BOOLEAN",
    ] = Field()


class AddCommandModel(BaseModel):
    cmd_id: str = Field()
    parent_id: str = Field()
    items_meta: list[FolderMeta | VarMeta] = Field()


class AddResponseModel(BaseModel):
    cmd_id: str = Field()


class ListCommandModel(BaseModel):
    cmd_id: str = Field()
    folder_id: str | None = Field(None)


class ListResponseModel(BaseModel):
    cmd_id: str = Field()
    children_folders: dict[str, str] = Field({})
    children_vars: dict[str, Any] = Field({})


class ValueModel(BaseModel):
    integer_value: int | None = Field(None)
    float_value: float | None = Field(None)
    text_value: str | None = Field(None)
    boolean_value: bool | None = Field(None)


class VarIdValueModel(BaseModel):
    var_id: str = Field()
    value: ValueModel = Field()


class SetCommandModel(BaseModel):
    cmd_id: str = Field()
    var_ids_values: list[VarIdValueModel] = Field()


class SetResponseModel(BaseModel):
    cmd_id: str = Field()


class GetCommandModel(BaseModel):
    cmd_id: str = Field()
    var_ids: list[str] = Field()


class GetResponseModel(BaseModel):
    cmd_id: str = Field()
    var_values: list = Field()


class DelCommandModel(BaseModel):
    cmd_id: str = Field()
    item_ids: list[str] = Field()


class DelResponseModel(BaseModel):
    cmd_id: str = Field()
