from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from typing import ClassVar as _ClassVar

DESCRIPTOR: _descriptor.FileDescriptor
ITEM_TYPE_FOLDER: ItemType
ITEM_TYPE_INVALID: ItemType
ITEM_TYPE_VARIABLE: ItemType
OPERATION_STATUS_ERR: OperationStatus
OPERATION_STATUS_INVALID: OperationStatus
OPERATION_STATUS_OK: OperationStatus
VAR_DATA_TYPE_BOOLEAN: VarDataType
VAR_DATA_TYPE_FLOAT: VarDataType
VAR_DATA_TYPE_INTEGER: VarDataType
VAR_DATA_TYPE_INVALID: VarDataType
VAR_DATA_TYPE_TEXT: VarDataType

class ItemType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = []

class VarDataType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = []

class OperationStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = []
