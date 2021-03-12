import gdal
from math import isclose
import numpy as N
import json

from pathlib import Path
from subprocess import check_output

MANIFEST_PATH = Path(__file__).parent.parent / "Cargo.toml"

def read_raster(path):
    raster = gdal.Open(str(path))
    x_size = raster.RasterXSize
    y_size = raster.RasterYSize
    count = raster.RasterCount
    data = []
    for i in range(count):
        bnd = raster.GetRasterBand(i+1)
        data.append(bnd.ReadAsArray(0, 0, x_size, y_size))
    return data


def create_raster(path, data):
    (height, width, bands) = data.shape

    if data.dtype == N.uint8:
        dtype = gdal.GDT_Byte
    else:
        dtype = gdal.GDT_Float64
        data = data.astype(N.float64)

    driver = gdal.GetDriverByName('GTiff')
    outRaster = driver.Create(str(path), width, height, bands, dtype)

    data = N.split(data, bands, axis=2)
    for i,arr in enumerate(data):
        outband = outRaster.GetRasterBand(i+1)
        outband.WriteArray(arr.reshape((height, width)))

    return outRaster

def create_random_raster(path, width, height, bands=1, dist=None):
    if dist is None: dist = N.random.normal

    data = dist(size=(height, width, bands))
    return create_raster(path, data)

def run_cargo(bin_name, *args, build=None):
    cargs = ['cargo', 'run', '--quiet', '--manifest-path', MANIFEST_PATH]
    if build: cargs.append( f'--{build}' )
    cargs += ['--bin', bin_name, '--']
    cargs += ['-c', str(1)]
    cargs += args
    output = check_output(cargs)
    if output:
        return json.loads(output)


def assert_is_close(a, b, desc=""):
    assert isclose(a, b, rel_tol=1e-2), f"{desc}: {a} == {b}"
