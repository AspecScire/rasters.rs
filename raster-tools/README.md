# raster-tools

Useful tools to process rasters. Most of these tools work on
a single-band float raster that represents a scalar field
(eg. a digital elevation model).

Install using `cargo-install`:

```
cargo install raster-tools
```

Provides the following binaries.

## raster-diff

Computes the difference raster between two rasters. The
geo-transforms, and the dimensions of the rasters can be
different; the two rasters are aligned, and the common
region is calculated. Provides options to compute the stats,
histogram of the difference, and/or create raster with the
difference. The output raster has the same extents, and
resolution as the first input and the data is the
no-data-value (`NAN`) outside the common region.

## raster-fill-nn

Computes and fill no-data-value of a raster using a
collection of points via natural-neighbors interpolation.
Uses the [spade](https://github.com/Stoeoef/spade) crate for
the interpolation.

## raster-stats

Computes first and second order stats (mean, min, max, std.
dev.) of a raster. Optionally the stats can be computed on
region contained inside each of a list of polygons. This is
similar to `gdalinfo -stats` but also allows restriction by
regions.

## raster-tile

Computes and write web mercator (EPSG:3857) tiles of a
raster. The output can be served as static files, and
displayed using map UI libraries like [ openlayers
](//openlayers.org/).

TODO: provide sample openlayers code to display tiles

## raster-mask

Computes a mask that represents the location where a raster
has data. For RGB rasters (i.e. if the input has 3 bands),
the no-data regions are where all values are 0 or the
no-data value of the first band. In other cases, the last
band is considered the mask.
