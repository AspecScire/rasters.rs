#!/usr/bin/env python3

import sys
from math import isclose

import json
def load_json(path):
    return json.load(open(path))

def compare_index(idx1, idx2, desc="root"):
    print(f"{desc}: {type(idx1)}, {type(idx2)}", file=sys.stderr)
    if type(idx1) != type(idx2):
        print(f"{desc}: types don't match")
        return

    if not isinstance(idx1, dict):
        if isinstance(idx1, float):
            if not isclose(idx1, idx2, rel_tol=1e-2):
                print(f"{desc} scalar values aren't close {idx1} == {idx2}")
        else:
            if idx1 != idx2:
                print(f"{desc} scalar values don't match {idx1} == {idx2}")
        return

    keys1 = list(idx1.keys())
    keys1.sort()
    keys2 = list(idx2.keys())
    keys2.sort()

    if keys1 != keys2:
        print(f"{desc} object keys do not match")
        print(keys1, keys2)
        return

    for k in keys1:
        compare_index(idx1[k], idx2[k], desc=f"{desc}/{k}")

def pretty_print_idx(idx):
    keys = list(idx.keys())
    keys.sort()

    xkeys = {}
    for k in keys:
        for xkey in idx[k]['index'].keys():
            xkeys[xkey] = True
    xkeys = list(xkeys.keys())
    xkeys.sort()
    # print(f"{'X':10}", end='')
    # for xkey in xkeys:
    #     print(f" {xkey:10}", end='')
    # print("")

    for k in keys:
        print(f"{k:10}", end='')
        inner = idx[k]['index']
        for xkey in xkeys:
            if xkey in inner:
                mn = inner[xkey]['min']
                if mn is not None:
                    print(f".", end='')
                else:
                    print(f"N", end='')
            else:
                print(f"X", end='')
        print("")


if __name__ == "__main__":
    # if len(sys.argv) < 3:
    #     print(f"Usage: {sys.argv[0]} <json1> <json2>", file=sys.stderr)
    #     sys.exit(1)

    file1 = sys.argv[1]
    idx = load_json(file1)
    zooms = [int(z) for z in idx.keys()]
    zooms.sort()
    for zoom in zooms:
        print(f"Zoom = {zoom}")
        pretty_print_idx(idx[str(zoom)])

    # file2 = sys.argv[2]
    # print(f"Comparing {file1} {file2}", file=sys.stderr)
    # compare_index(load_json(file1), load_json(file2))
