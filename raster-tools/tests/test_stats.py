from .harness import create_random_raster, run_cargo, create_raster, assert_is_close
from tempfile import TemporaryDirectory

from pathlib import Path
import numpy as N
from math import sqrt

with TemporaryDirectory(prefix="test-raster-stats-") as base_path:
    base_path = Path(base_path)

    raster_path = base_path / "raster.tif"

    data = N.random.normal(size=(64, 64, 1))
    create_raster(raster_path, data)

    stats = run_cargo('raster-stats', str(raster_path))[0]

    assert_is_close(stats['max'], N.max(data), desc='max')
    assert_is_close(stats['min'], N.min(data), desc='min')
    assert_is_close(stats['sum'], N.sum(data), desc='sum')
    assert_is_close(sqrt(stats['sum_2'] / stats['count']), N.std(data), desc='std')

print("Test raster-stats succeeded")
