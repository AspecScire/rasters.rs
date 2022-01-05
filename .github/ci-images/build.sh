#!/bin/sh

echo "Building docker image..."
echo "Using GDAL" ${GDAL_VERSION:?}
echo "Using RUST" ${RUST_VERSION:?}

IMAGE_NAME=rmanoka/georust-ci:gdal-"${GDAL_VERSION}"-rust-"${RUST_VERSION}"

docker build -t "${IMAGE_NAME}" --build-arg=GDAL_VERSION="${GDAL_VERSION}" --build-arg=RUST_VERSION="${RUST_VERSION}" .

echo "Checking gdal version in image"
docker run --rm "${IMAGE_NAME}" gdalinfo --version

echo "Checking rust version in image"
docker run --rm "${IMAGE_NAME}" rustc --version
