from .harness import create_random_raster, run_cargo, create_raster, assert_is_close, read_raster
from tempfile import TemporaryDirectory

from pathlib import Path
import numpy as N
from math import sqrt

import json
def load_json(path):
    return json.load(open(path))

def compare_index(idx1, idx2, desc="root"):
    assert type(idx1) == type(idx2), f"index type @ {desc} matches reference"

    if not isinstance(idx1, dict):
        if isinstance(idx1, int):
            assert idx1 == idx2, f"int value {idx1} == {idx2}"
        else:
            assert_is_close(idx1, idx2)
        return

    keys1 = list(idx1.keys())
    keys1.sort()
    keys2 = list(idx2.keys())
    keys2.sort()

    assert keys1 == keys2, f"index keys @ {desc} matches reference"
    for k in keys1:
        compare_index(idx1[k], idx2[k], desc=f"{desc}/{k}")



with TemporaryDirectory(prefix="test-raster-tile-") as base_path:
    # This test verifies against fixtures generated against a specific seed
    N.random.seed(0xfab1)

    base_path = Path(base_path)

    raster_path = base_path / "raster.tif"
    data = N.random.normal(size=(64, 64, 1))
    raster = create_raster(raster_path, data)
    raster.SetGeoTransform( (363737.54688808107, 0.08917409880025007, 0.0, 2059515.3774022115, 0.0, -0.08917409880023142) )
    raster.SetProjection('PROJCS["WGS 84 / UTM zone 43N",GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563,AUTHORITY["EPSG","7030"]],AUTHORITY["EPSG","6326"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["degree",0.0174532925199433,AUTHORITY["EPSG","9122"]],AUTHORITY["EPSG","4326"]],PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",0],PARAMETER["central_meridian",75],PARAMETER["scale_factor",0.9996],PARAMETER["false_easting",500000],PARAMETER["false_northing",0],UNIT["metre",1,AUTHORITY["EPSG","9001"]],AXIS["Easting",EAST],AXIS["Northing",NORTH],AUTHORITY["EPSG","32643"]]')
    raster = None

    out_path = base_path / "tiles"
    run_cargo('raster-tile', str(raster_path), str(out_path))

    tile_idx = load_json(out_path / "index.json")
    ref_idx = load_json(Path(__file__).parent / "fixtures" / "tile-test-fab1-index.json")
    compare_index(tile_idx, ref_idx)

print("Test raster-tile succeeded")
