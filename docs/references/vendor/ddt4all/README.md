# DDT4All reference snapshot (not a runtime dependency)

Source: https://github.com/cedricp/ddt4all/tree/09a39795725b7e510041436cdde3b1d65609142c
License: GPL-3.0-or-later; original license reproduced in license.txt. Copyright remains with the original authors and contributors (see upstream history). Files copied unmodified; expected Git blob SHA listed below. This selective snapshot is for porting and test design, not an installed Python package, and must not be executed against a vehicle. Dependencies and original context may be absent. No Python build/runtime integration; no macros, vehicle writes or security bypass. Any Rust port requires tests, appropriate attribution, safe policy checks and review.

| Original path | Upstream Git blob SHA |
|---|---|
| `license.txt` | `94a9ed024d3859793618152ea559a168bbcbb5e2` |
| `src/ddt4all/core/ecu/data_item.py` | `212eca29abd661cbe081a34f81ae939783aad08e` |
| `src/ddt4all/core/ecu/ecu_ident.py` | `7c5c1555c13ad04c0953b6a8e370ee1c742c5695` |
| `src/ddt4all/core/ecu/ecu_database.py` | `1a3e4a7bca5886c206783ab7db8887a25af0012a` |
| `src/ddt4all/core/elm/device_manager.py` | `6cebb40a43b860309d93c13527bc7eba25e1ad28` |
| `src/ddt4all/core/doip/doip_message_type.py` | `c3ff60e77cf814e330ec60f467acaa1d397e1c5c` |
| `src/ddt4all/core/doip/doip_protocol_error.py` | `622f6d78a3cbba7355e48a5f8fb2ec5091972d2c` |
| `src/ddt4all/core/doip/doip_connection.py` | `b874e7391367302cffcda468bfaa5b1b8826c7fa` |
| `tests/resources/chatgpt_generated/dummy_ecu_can.json` | `fa587681188cb8a374c402cc72e38be8808c97b9` |
| `tests/resources/chatgpt_generated/dummy_ecu_can.xml` | `d266452565dfcf953c3b4d59d72fa1b89e5d3edd` |
| `tests/resources/chatgpt_generated/dummy_ecu_can_ext.xml` | `64385fa97488a5ef4adc5625d9ab9cabac8fa179` |
| `tests/resources/chatgpt_generated/dummy_ecu_iso8.xml` | `3ce8c25d73a29680e79aa3bdde5e340397c153ae` |
| `tests/resources/chatgpt_generated/dummy_ecu_kwp.xml` | `32496c60e733b1cb54ef65180af21c7a16910316` |

## Database scope

The five copied XML/JSON files are upstream synthetic test fixtures, not a manufacturer ECU database. The actual ECU database is loaded externally by DDT4All from ecu.zip/db.json, XML or local JSON; ecu.zip is absent in the pinned tree. The public tree includes src/ddt4all/resources/projects.json (5,268,760 bytes, blob fdb1be422e26cd0c0b117da4db6125e049e25982), but its commit history includes https://github.com/cedricp/ddt4all/commit/fe3f97956cc4c7f3fd40925eca97291872a89e10 ("update project json data for uses recent ecus.zip"). Independent authorization for redistributing the underlying data has not been established; do not vendor or ship it until verified. The GitHub file retrieval also did not yield its contents in the inspected line range. Do not misrepresent synthetic fixtures as OEM coverage.

See [technical reference](../../DDT4ALL.md) and [roadmap](../../../ROADMAP.md). No claims of implementation, complete upstream mirroring or physical compatibility.
