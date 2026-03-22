from . import enums_pb2 as _enums_pb2
from . import types_pb2 as _types_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from collections.abc import Iterable as _Iterable, Mapping as _Mapping
from typing import ClassVar as _ClassVar, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class FolderChanged(_message.Message):
    __slots__ = ("folder_id", "reload", "removed_items", "new_folders", "new_variables")
    FOLDER_ID_FIELD_NUMBER: _ClassVar[int]
    RELOAD_FIELD_NUMBER: _ClassVar[int]
    REMOVED_ITEMS_FIELD_NUMBER: _ClassVar[int]
    NEW_FOLDERS_FIELD_NUMBER: _ClassVar[int]
    NEW_VARIABLES_FIELD_NUMBER: _ClassVar[int]
    folder_id: str
    reload: bool
    removed_items: _containers.RepeatedScalarFieldContainer[str]
    new_folders: _containers.RepeatedCompositeFieldContainer[_types_pb2.FolderInfo]
    new_variables: _containers.RepeatedCompositeFieldContainer[_types_pb2.VarInfo]
    def __init__(self, folder_id: _Optional[str] = ..., reload: _Optional[bool] = ..., removed_items: _Optional[_Iterable[str]] = ..., new_folders: _Optional[_Iterable[_Union[_types_pb2.FolderInfo, _Mapping]]] = ..., new_variables: _Optional[_Iterable[_Union[_types_pb2.VarInfo, _Mapping]]] = ...) -> None: ...

class TreeChanged(_message.Message):
    __slots__ = ("folder_changed_event",)
    FOLDER_CHANGED_EVENT_FIELD_NUMBER: _ClassVar[int]
    folder_changed_event: _containers.RepeatedCompositeFieldContainer[FolderChanged]
    def __init__(self, folder_changed_event: _Optional[_Iterable[_Union[FolderChanged, _Mapping]]] = ...) -> None: ...

class Event(_message.Message):
    __slots__ = ("var_value_ev", "tree_changed_ev")
    VAR_VALUE_EV_FIELD_NUMBER: _ClassVar[int]
    TREE_CHANGED_EV_FIELD_NUMBER: _ClassVar[int]
    var_value_ev: _types_pb2.VarIdValue
    tree_changed_ev: TreeChanged
    def __init__(self, var_value_ev: _Optional[_Union[_types_pb2.VarIdValue, _Mapping]] = ..., tree_changed_ev: _Optional[_Union[TreeChanged, _Mapping]] = ...) -> None: ...
