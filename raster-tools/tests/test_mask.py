from .harness import create_random_raster, run_cargo, create_raster, assert_is_close, read_raster
from tempfile import TemporaryDirectory

from pathlib import Path
import numpy as N
from math import sqrt

with TemporaryDirectory(prefix="test-raster-mask-") as base_path:
    base_path = Path(base_path)

    raster_path = base_path / "raster.tif"
    out_path = base_path / "mask.tif"
    data = N.random.randint(low=2, size=(64, 64, 3))
    create_raster(raster_path, data)

    run_cargo('raster-mask', str(raster_path), str(out_path))

    odata = read_raster(str(out_path))[0]

    assert odata.shape == data.shape[:2], f"output shape {odata.shape} == input shape {data.shape[:2]}"
    cdata = N.all(data == 0, axis=2).astype(N.uint8)
    odata = odata.astype(N.bool).astype(N.uint8)
    assert N.all((cdata + odata) == 1), f"mask is correct"

print("Test raster-mask succeeded")
