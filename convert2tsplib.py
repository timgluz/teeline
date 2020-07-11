#!/bin/python

# A script that converts 2D coord points into TSPLIB format
# usage:
#   python convert2tsplib ./data/

import sys
from pathlib import Path
from collections import namedtuple

TsplibData = namedtuple('TsplibData', ['name', 'comment', 'cities'])


def read_discopt_file(filepath):
    filename = filepath.name
    comment = f"converted from DiscOpt dataset {filename}"

    line_no = 1
    cities = []
    with filepath.open('r') as f:
        # ignore first line
        f.readline()

        for line in f:
            (x, y) = line.split()
            cities.append((line_no, x, y))
            line_no += 1
    return TsplibData(filename, comment, cities)


def write_tsplib_file(out_filepath, tsp_dt):
    with out_filepath.open('w') as f:
        f.write(f"NAME: {tsp_dt.name}\n")
        f.write("TYPE: TSP\n")
        f.write(f"COMMENT: {tsp_dt.comment}\n")
        f.write(f"DIMENSION: {len(tsp_dt.cities)}\n")
        f.write("EDGE_WEIGHT_TYPE: EUC_2D\n")
        f.write("NODE_COORD_SECTION\n")
        for city in tsp_dt.cities:
            f.write(f"\t{city[0]} {city[1]} {city[2]}\n")

        f.write("EOF\n")


def convert2tsplib(filepath):
    tsp_dt = read_discopt_file(filepath)

    out_filepath = Path(f"./data/discopt/{filepath.stem}.tsp")
    write_tsplib_file(out_filepath, tsp_dt)
    return out_filepath


def convert_all_files(source_folder):
    for filepath in source_folder.iterdir():
        if filepath.is_file() is False:
            continue

        print(f"Converting: {filepath}\n")
        convert2tsplib(filepath)


if __name__ == "__main__":
    source_path = sys.argv[1]
    if source_path is None:
        print("Missing filepath. usage: convert2tsplib <FOLDER>\n")
        exit(1)

    source_path = Path(source_path)
    if not source_path.exists():
        print(f"Source folder doesnt exists: {source_path}\n")
        exit(1)

    convert_all_files(source_path)
    print("Done.\n")
