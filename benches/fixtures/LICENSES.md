# Third-Party Benchmark Fixtures

This directory contains JSON files from third-party sources used for benchmarking.
Each file is used under its respective license with attribution as required.

## india-osm.geojson / india-composite.geojson / india-soi.geojson

- **Source**: [datameet/maps](https://github.com/datameet/maps/tree/master/Country)
- **Attribution**: DataMeet Community (http://datameet.org/)
- **Description**:
  - `india-osm.geojson` (6.3MB) - India's boundary from OpenStreetMap (ODbL)
  - `india-composite.geojson` (11MB) - India's boundary from composite sources (CC-0)
  - `india-soi.geojson` (12MB) - India's boundary from Survey of India/Census 2011 (CC-by-sa 2.5/ODbL)

Licenses: ODbL, CC-0, CC-by-sa 2.5 (see source repo for details)

## vscode-*-package-lock.json

- **Source**: [microsoft/vscode](https://github.com/microsoft/vscode)
- **License**: MIT
- **Copyright**: Copyright (c) Microsoft Corporation
- **Description**: VSCode package-lock.json files from different versions for graduated diff testing
  - `vscode-1.94-package-lock.json` (751KB) - Baseline (oldest with package-lock.json)
  - `vscode-1.95-package-lock.json` (702KB) - ~18% diff from 1.94
  - `vscode-1.100-package-lock.json` (708KB) - ~8% diff from 1.95
  - `vscode-1.108-package-lock.json` (681KB) - Latest release, ~42% diff from 1.94
