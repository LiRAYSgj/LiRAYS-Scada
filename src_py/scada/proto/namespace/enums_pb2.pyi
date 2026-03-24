from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from typing import ClassVar as _ClassVar

DESCRIPTOR: _descriptor.FileDescriptor

class ItemType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ITEM_TYPE_INVALID: _ClassVar[ItemType]
    ITEM_TYPE_FOLDER: _ClassVar[ItemType]
    ITEM_TYPE_VARIABLE: _ClassVar[ItemType]

class VarDataType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    VAR_DATA_TYPE_INVALID: _ClassVar[VarDataType]
    VAR_DATA_TYPE_INTEGER: _ClassVar[VarDataType]
    VAR_DATA_TYPE_FLOAT: _ClassVar[VarDataType]
    VAR_DATA_TYPE_TEXT: _ClassVar[VarDataType]
    VAR_DATA_TYPE_BOOLEAN: _ClassVar[VarDataType]

class OperationStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    OPERATION_STATUS_INVALID: _ClassVar[OperationStatus]
    OPERATION_STATUS_OK: _ClassVar[OperationStatus]
    OPERATION_STATUS_ERR: _ClassVar[OperationStatus]

class EventType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    EVENT_TYPE_INVALID: _ClassVar[EventType]
    EVENT_TYPE_TREE_CHANGE: _ClassVar[EventType]
    EVENT_TYPE_VAR_VALUES: _ClassVar[EventType]
ITEM_TYPE_INVALID: ItemType
ITEM_TYPE_FOLDER: ItemType
ITEM_TYPE_VARIABLE: ItemType
VAR_DATA_TYPE_INVALID: VarDataType
VAR_DATA_TYPE_INTEGER: VarDataType
VAR_DATA_TYPE_FLOAT: VarDataType
VAR_DATA_TYPE_TEXT: VarDataType
VAR_DATA_TYPE_BOOLEAN: VarDataType
OPERATION_STATUS_INVALID: OperationStatus
OPERATION_STATUS_OK: OperationStatus
OPERATION_STATUS_ERR: OperationStatus
EVENT_TYPE_INVALID: EventType
EVENT_TYPE_TREE_CHANGE: EventType
EVENT_TYPE_VAR_VALUES: EventType
