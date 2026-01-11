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

## citm_catalog.json

- **Source**: [miloyip/nativejson-benchmark](https://github.com/miloyip/nativejson-benchmark/blob/master/data/citm_catalog.json)
- **License**: MIT
- **Copyright**: Copyright (c) 2014-2016 Milo Yip
- **Description**: Event catalog data used in Java JSON parser benchmarks

## twitter.json

- **Source**: [miloyip/nativejson-benchmark](https://github.com/miloyip/nativejson-benchmark/blob/master/data/twitter.json)
- **License**: MIT
- **Copyright**: Copyright (c) 2014-2016 Milo Yip
- **Description**: Twitter API response with CJK (Chinese/Japanese/Korean) text

## vscode-*-package-lock.json

- **Source**: [microsoft/vscode](https://github.com/microsoft/vscode)
- **License**: MIT
- **Copyright**: Copyright (c) Microsoft Corporation
- **Description**: VSCode package-lock.json files from different versions for graduated diff testing
  - `vscode-1.94-package-lock.json` (751KB) - Baseline (oldest with package-lock.json)
  - `vscode-1.95-package-lock.json` (702KB) - ~18% diff from 1.94
  - `vscode-1.100-package-lock.json` (708KB) - ~8% diff from 1.95
  - `vscode-1.108-package-lock.json` (681KB) - Latest release, ~42% diff from 1.94

---

## MIT License (nativejson-benchmark)

```
MIT License

Copyright (c) 2014-2016 Milo Yip

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
