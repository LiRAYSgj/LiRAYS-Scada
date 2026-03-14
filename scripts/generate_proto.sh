#!/bin/bash
set -e
cd "$(dirname "$0")/.."

mkdir -p src/scada/proto/namespace
touch src/scada/proto/__init__.py
touch src/scada/proto/namespace/__init__.py

protoc --python_out=src/scada/proto --pyi_out=src/scada/proto -I=rust/proto rust/proto/namespace/*.proto

# Fix python relative imports
for file in src/scada/proto/namespace/*_pb2.py src/scada/proto/namespace/*_pb2.pyi; do
    sed -i '' 's/from namespace import/from . import/g' "$file"
done

echo "Python protobuf files generated successfully!"
