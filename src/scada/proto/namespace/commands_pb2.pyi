from . import enums_pb2 as _enums_pb2
from . import types_pb2 as _types_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from collections.abc import Iterable as _Iterable, Mapping as _Mapping
from typing import ClassVar as _ClassVar, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class AddCommand(_message.Message):
    __slots__ = ("cmd_id", "parent_id", "items_meta")
    CMD_ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_ID_FIELD_NUMBER: _ClassVar[int]
    ITEMS_META_FIELD_NUMBER: _ClassVar[int]
    cmd_id: str
    parent_id: str
    items_meta: _containers.RepeatedCompositeFieldContainer[_types_pb2.ItemMeta]
    def __init__(self, cmd_id: _Optional[str] = ..., parent_id: _Optional[str] = ..., items_meta: _Optional[_Iterable[_Union[_types_pb2.ItemMeta, _Mapping]]] = ...) -> None: ...

class AddResponse(_message.Message):
    __slots__ = ("cmd_id",)
    CMD_ID_FIELD_NUMBER: _ClassVar[int]
    cmd_id: str
    def __init__(self, cmd_id: _Optional[str] = ...) -> None: ...

class AddBulkCommand(_message.Message):
    __slots__ = ("cmd_id", "parent_id", "schema")
    CMD_ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_ID_FIELD_NUMBER: _ClassVar[int]
    SCHEMA_FIELD_NUMBER: _ClassVar[int]
    cmd_id: str
    parent_id: str
    schema: _types_pb2.NamespaceSchema
    def __init__(self, cmd_id: _Optional[str] = ..., parent_id: _Optional[str] = ..., schema: _Optional[_Union[_types_pb2.NamespaceSchema, _Mapping]] = ...) -> None: ...

class AddBulkResponse(_message.Message):
    __slots__ = ("cmd_id",)
    CMD_ID_FIELD_NUMBER: _ClassVar[int]
    cmd_id: str
    def __init__(self, cmd_id: _Optional[str] = ...) -> None: ...

class ListCommand(_message.Message):
    __slots__ = ("cmd_id", "folder_id")
    CMD_ID_FIELD_NUMBER: _ClassVar[int]
    FOLDER_ID_FIELD_NUMBER: _ClassVar[int]
    cmd_id: str
    folder_id: str
    def __init__(self, cmd_id: _Optional[str] = ..., folder_id: _Optional[str] = ...) -> None: ...

class ListResponse(_message.Message):
    __slots__ = ("cmd_id", "children_folders", "children_vars")
    class ChildrenFoldersEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: str
        def __init__(self, key: _Optional[str] = ..., value: _Optional[str] = ...) -> None: ...
    class ChildrenVarsEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: _types_pb2.VarInfo
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[_types_pb2.VarInfo, _Mapping]] = ...) -> None: ...
    CMD_ID_FIELD_NUMBER: _ClassVar[int]
    CHILDREN_FOLDERS_FIELD_NUMBER: _ClassVar[int]
    CHILDREN_VARS_FIELD_NUMBER: _ClassVar[int]
    cmd_id: str
    children_folders: _containers.ScalarMap[str, str]
    children_vars: _containers.MessageMap[str, _types_pb2.VarInfo]
    def __init__(self, cmd_id: _Optional[str] = ..., children_folders: _Optional[_Mapping[str, str]] = ..., children_vars: _Optional[_Mapping[str, _types_pb2.VarInfo]] = ...) -> None: ...

class SetCommand(_message.Message):
    __slots__ = ("cmd_id", "var_ids_values")
    CMD_ID_FIELD_NUMBER: _ClassVar[int]
    VAR_IDS_VALUES_FIELD_NUMBER: _ClassVar[int]
    cmd_id: str
    var_ids_values: _containers.RepeatedCompositeFieldContainer[_types_pb2.VarIdValue]
    def __init__(self, cmd_id: _Optional[str] = ..., var_ids_values: _Optional[_Iterable[_Union[_types_pb2.VarIdValue, _Mapping]]] = ...) -> None: ...

class SetResponse(_message.Message):
    __slots__ = ("cmd_id",)
    CMD_ID_FIELD_NUMBER: _ClassVar[int]
    cmd_id: str
    def __init__(self, cmd_id: _Optional[str] = ...) -> None: ...

class GetCommand(_message.Message):
    __slots__ = ("cmd_id", "var_ids")
    CMD_ID_FIELD_NUMBER: _ClassVar[int]
    VAR_IDS_FIELD_NUMBER: _ClassVar[int]
    cmd_id: str
    var_ids: _containers.RepeatedScalarFieldContainer[str]
    def __init__(self, cmd_id: _Optional[str] = ..., var_ids: _Optional[_Iterable[str]] = ...) -> None: ...

class GetResponse(_message.Message):
    __slots__ = ("cmd_id", "var_values")
    CMD_ID_FIELD_NUMBER: _ClassVar[int]
    VAR_VALUES_FIELD_NUMBER: _ClassVar[int]
    cmd_id: str
    var_values: _containers.RepeatedCompositeFieldContainer[_types_pb2.OptionalValue]
    def __init__(self, cmd_id: _Optional[str] = ..., var_values: _Optional[_Iterable[_Union[_types_pb2.OptionalValue, _Mapping]]] = ...) -> None: ...

class DelCommand(_message.Message):
    __slots__ = ("cmd_id", "item_ids")
    CMD_ID_FIELD_NUMBER: _ClassVar[int]
    ITEM_IDS_FIELD_NUMBER: _ClassVar[int]
    cmd_id: str
    item_ids: _containers.RepeatedScalarFieldContainer[str]
    def __init__(self, cmd_id: _Optional[str] = ..., item_ids: _Optional[_Iterable[str]] = ...) -> None: ...

class DelResponse(_message.Message):
    __slots__ = ("cmd_id",)
    CMD_ID_FIELD_NUMBER: _ClassVar[int]
    cmd_id: str
    def __init__(self, cmd_id: _Optional[str] = ...) -> None: ...

class InvalidCmdResponse(_message.Message):
    __slots__ = ("cmd_id",)
    CMD_ID_FIELD_NUMBER: _ClassVar[int]
    cmd_id: str
    def __init__(self, cmd_id: _Optional[str] = ...) -> None: ...

class Command(_message.Message):
    __slots__ = ("add", "list", "set", "get", "add_bulk")
    ADD_FIELD_NUMBER: _ClassVar[int]
    LIST_FIELD_NUMBER: _ClassVar[int]
    SET_FIELD_NUMBER: _ClassVar[int]
    GET_FIELD_NUMBER: _ClassVar[int]
    DEL_FIELD_NUMBER: _ClassVar[int]
    ADD_BULK_FIELD_NUMBER: _ClassVar[int]
    add: AddCommand
    list: ListCommand
    set: SetCommand
    get: GetCommand
    add_bulk: AddBulkCommand
    def __init__(self, add: _Optional[_Union[AddCommand, _Mapping]] = ..., list: _Optional[_Union[ListCommand, _Mapping]] = ..., set: _Optional[_Union[SetCommand, _Mapping]] = ..., get: _Optional[_Union[GetCommand, _Mapping]] = ..., add_bulk: _Optional[_Union[AddBulkCommand, _Mapping]] = ..., **kwargs) -> None: ...

class Response(_message.Message):
    __slots__ = ("add", "list", "set", "get", "inv", "add_bulk", "status", "error_msg")
    ADD_FIELD_NUMBER: _ClassVar[int]
    LIST_FIELD_NUMBER: _ClassVar[int]
    SET_FIELD_NUMBER: _ClassVar[int]
    GET_FIELD_NUMBER: _ClassVar[int]
    DEL_FIELD_NUMBER: _ClassVar[int]
    INV_FIELD_NUMBER: _ClassVar[int]
    ADD_BULK_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    ERROR_MSG_FIELD_NUMBER: _ClassVar[int]
    add: AddResponse
    list: ListResponse
    set: SetResponse
    get: GetResponse
    inv: InvalidCmdResponse
    add_bulk: AddBulkResponse
    status: _enums_pb2.OperationStatus
    error_msg: str
    def __init__(self, add: _Optional[_Union[AddResponse, _Mapping]] = ..., list: _Optional[_Union[ListResponse, _Mapping]] = ..., set: _Optional[_Union[SetResponse, _Mapping]] = ..., get: _Optional[_Union[GetResponse, _Mapping]] = ..., inv: _Optional[_Union[InvalidCmdResponse, _Mapping]] = ..., add_bulk: _Optional[_Union[AddBulkResponse, _Mapping]] = ..., status: _Optional[_Union[_enums_pb2.OperationStatus, str]] = ..., error_msg: _Optional[str] = ..., **kwargs) -> None: ...
