from .harness import create_random_raster, run_cargo, create_raster, assert_is_close
from tempfile import TemporaryDirectory

from pathlib import Path
import numpy as N
from math import sqrt

with TemporaryDirectory(prefix="test-raster-diff-") as base_path:
    base_path = Path(base_path)

    raster1_path = base_path / "raster1.tif"
    data1 = N.random.normal(size=(64, 64, 1))
    create_raster(raster1_path, data1).SetGeoTransform([-32., 1., 0., -32., 0., 1.])

    raster2_path = base_path / "raster2.tif"
    data2 = N.random.normal(size=(32, 32, 1))
    create_raster(raster2_path, data2).SetGeoTransform([-64., 4., 0., -64., 0., 4.])

    stats = run_cargo('raster-diff', str(raster1_path), str(raster2_path))['stats']['diff']
    diff = data2[8:-8, 8:-8, 0].repeat(4, axis=0).repeat(4, axis=1) - data1[:,:,0]

    assert_is_close(stats['max'], N.max(diff), desc='max')
    assert_is_close(stats['min'], N.min(diff), desc='min')
    assert_is_close(stats['sum'], N.sum(diff), desc='sum')

print("Test raster-diff succeeded")
