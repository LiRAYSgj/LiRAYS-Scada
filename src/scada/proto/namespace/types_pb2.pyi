from . import enums_pb2 as _enums_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from collections.abc import Mapping as _Mapping
from typing import ClassVar as _ClassVar, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class Meta(_message.Message):
    __slots__ = ("root_uid", "vendor")
    ROOT_UID_FIELD_NUMBER: _ClassVar[int]
    VENDOR_FIELD_NUMBER: _ClassVar[int]
    root_uid: str
    vendor: str
    def __init__(self, root_uid: _Optional[str] = ..., vendor: _Optional[str] = ...) -> None: ...

class Value(_message.Message):
    __slots__ = ("integer_value", "float_value", "text_value", "boolean_value")
    INTEGER_VALUE_FIELD_NUMBER: _ClassVar[int]
    FLOAT_VALUE_FIELD_NUMBER: _ClassVar[int]
    TEXT_VALUE_FIELD_NUMBER: _ClassVar[int]
    BOOLEAN_VALUE_FIELD_NUMBER: _ClassVar[int]
    integer_value: int
    float_value: float
    text_value: str
    boolean_value: bool
    def __init__(self, integer_value: _Optional[int] = ..., float_value: _Optional[float] = ..., text_value: _Optional[str] = ..., boolean_value: _Optional[bool] = ...) -> None: ...

class OptionalValue(_message.Message):
    __slots__ = ("value",)
    VALUE_FIELD_NUMBER: _ClassVar[int]
    value: Value
    def __init__(self, value: _Optional[_Union[Value, _Mapping]] = ...) -> None: ...

class ChildInfo(_message.Message):
    __slots__ = ("child_id", "i_type", "var_d_type")
    CHILD_ID_FIELD_NUMBER: _ClassVar[int]
    I_TYPE_FIELD_NUMBER: _ClassVar[int]
    VAR_D_TYPE_FIELD_NUMBER: _ClassVar[int]
    child_id: str
    i_type: _enums_pb2.ItemType
    var_d_type: _enums_pb2.VarDataType
    def __init__(self, child_id: _Optional[str] = ..., i_type: _Optional[_Union[_enums_pb2.ItemType, str]] = ..., var_d_type: _Optional[_Union[_enums_pb2.VarDataType, str]] = ...) -> None: ...

class Item(_message.Message):
    __slots__ = ("id", "name", "parent", "children", "i_type", "var_d_type", "value")
    class ChildrenEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ChildInfo
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ChildInfo, _Mapping]] = ...) -> None: ...
    ID_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    PARENT_FIELD_NUMBER: _ClassVar[int]
    CHILDREN_FIELD_NUMBER: _ClassVar[int]
    I_TYPE_FIELD_NUMBER: _ClassVar[int]
    VAR_D_TYPE_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    id: str
    name: str
    parent: str
    children: _containers.MessageMap[str, ChildInfo]
    i_type: _enums_pb2.ItemType
    var_d_type: _enums_pb2.VarDataType
    value: Value
    def __init__(self, id: _Optional[str] = ..., name: _Optional[str] = ..., parent: _Optional[str] = ..., children: _Optional[_Mapping[str, ChildInfo]] = ..., i_type: _Optional[_Union[_enums_pb2.ItemType, str]] = ..., var_d_type: _Optional[_Union[_enums_pb2.VarDataType, str]] = ..., value: _Optional[_Union[Value, _Mapping]] = ...) -> None: ...
