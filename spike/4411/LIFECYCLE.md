# ESP32-C3-WROOM-02 lifecycle (Espressif-published sources), retrieved 2026-11-24

## Verdict
HELD: "Mass Production" (Espressif product-selector data) for all six ordering codes below.
No Espressif PCN/NRND/EOL notice naming the module was found. Absence of a notice is NOT the basis of the verdict; the basis is the positive status field below.
Caveat: the selector status is a live database field, not a signed or dated lifecycle statement. The longevity page names no module. The selector's own status vocabulary: Mass Production, EOL, NRND, Replaced, Sample.

## Quote (source JSON, key path results[i].status, from `chipselection` backend of the Product Selector)
File api_products.json, object keys "results" -> array element; each record has "name", "mpn", "status":
- id 60  "name": "ESP32-C3-WROOM-02",  "mpn": "ESP32-C3-WROOM-02-N4",  "status": "Mass Production"  (results[68], releaseTime 2020/01/01)
- id 61  "name": "ESP32-C3-WROOM-02",  "mpn": "ESP32-C3-WROOM-02-H4",  "status": "Mass Production"  (results[69], 2021/01/01)
- id 90  "name": "ESP32-C3-WROOM-02U", "mpn": "ESP32-C3-WROOM-02U-N4", "status": "Mass Production"  (results[70], 2021/1/1)
- id 91  "name": "ESP32-C3-WROOM-02U", "mpn": "ESP32-C3-WROOM-02U-H4", "status": "Mass Production"  (results[71], 2021/1/1)
- id 240 "mpn": "ESP32-C3-WROOM-02-N8"  "status": "Mass Production" (results[284]; extra code, not in the WO list)
- id 241 "mpn": "ESP32-C3-WROOM-02U-N8" "status": "Mass Production" (results[285]; extra code)

## Primary source
- URL: https://products.espressif.com/api/user/products  (the JSON the product selector SPA at https://products.espressif.com/ loads; path found in its JS chunk 0: baseURL "/api", "/user/products")
- Retrieved: 2026-11-24
- File: /home/forge/.cache/aet-4411/lifecycle/api_products.json
- sha256: 942911652374043c3fc958ec91507ed711777b2867db78f5e517c4fb07b2f157
- Re-check: python3 -I -c "import json;[print(x['mpn'],x['status']) for x in json.load(open('api_products.json'))['results'] if 'C3-WROOM-02' in x['mpn']]"

## Other held files (all retrieved 2026-11-24, curl -fsSL)
| URL | file | sha256 | what it showed |
|---|---|---|---|
| https://products.espressif.com/api/user/products/selection | api_products_selection.json | e97e1e87b348f3b92dccd703e049c2280d54117c54c25069a6543c4cf15c1887 | id/name/mpn list only; confirms the 4 ordering codes (+N8) exist; no status |
| https://www.espressif.com/en/products/longevity-commitment | longevity.html | c28259623b3dff2f95c01c5f46d4c919500ad69863a856057e01960b7fe7c67a | generic "At Least 12 Years" commitment text (per chip series); no mention of ESP32-C3-WROOM-02 module (only ESP32-C3 menu/link text) |
| https://www.espressif.com/en/support/documents/pcns | pcns.html | d0feca9ffb85cabc29b2b98d3fdf19e72663fc063557e46ce8bab802cab36a6a | no occurrence of "C3-WROOM", no NRND / "end of life" strings |
| https://www.espressif.com/en/support/documents/advisories | advisories.html | f92236f5d597d7ca7f1e61c68e36ed9aad4e5f28d0b88b0b6aa42c63da91e8ff | ESP32-C3 chip advisories (chip revision v1.1 etc.); no "C3-WROOM" module notice |

## URLs checked
- https://products.espressif.com/  (SPA shell, no data; led to the API)
- https://products.espressif.com/static/js/app.d85b608528ea0935da1f.js and chunks 0-7 (to find /api endpoints; also saved here)
- https://products.espressif.com/api/user/products (HELD, above), /api/user/products/selection (held), /api/user/products/query and /select (HTTP 405 on GET, not pursued)
- https://www.espressif.com/en/products/longevity-commitment, /en/support/documents/pcns, /en/support/documents/advisories (above)
- https://docs.espressif.com/projects/esp-idf/en/latest/esp32c3/ (HTTP 200, not saved; software docs, no module lifecycle)
- https://www.espressif.com/sites/default/files/documentation/esp32-c3-wroom-02_datasheet_en.pdf (HTTP 200 only, not downloaded or read; not relied on)
- Distributors (DigiKey, Mouser): not held, not consulted.
