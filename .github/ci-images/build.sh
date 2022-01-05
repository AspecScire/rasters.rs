#!/bin/sh

echo "Building docker image..."
echo "Using GDAL" ${GDAL_VERSION:?}
echo "Using RUST" ${RUST_VERSION:?}
