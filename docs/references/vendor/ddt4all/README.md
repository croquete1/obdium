# Third-party data fixtures and source provenance (Rust-only integration)

Source: https://github.com/cedricp/ddt4all/tree/09a39795725b7e510041436cdde3b1d65609142c
License: GPL-3.0-or-later; original license reproduced in license.txt. Copyright remains with the original authors and contributors (see upstream history). The original GPL license, five synthetic XML/JSON fixtures, 15 translation catalogs and one English POT template are retained in the current tree; their upstream blob hashes are listed below. The previously copied Python source modules were removed from the proposed tree; their hashes are retained for provenance only and they can be inspected via the pinned upstream URL. No Python source, interpreter, bridge, subprocess, build/runtime/test dependency or dependency stack from the reference project may be added for this integration. Any Rust port requires tests, appropriate attribution, safe policy checks and review.

| Original path | Upstream Git blob SHA | Current tree |
|---|---|---|
| `license.txt` | `94a9ed024d3859793618152ea559a168bbcbb5e2`  Retained |
| `src/ddt4all/core/ecu/data_item.py` | `212eca29abd661cbe081a34f81ae939783aad08e`  Removed; provenance only |
| `src/ddt4all/core/ecu/ecu_ident.py` | `7c5c1555c13ad04c0953b6a8e370ee1c742c5695`  Removed; provenance only |
| `src/ddt4all/core/ecu/ecu_database.py` | `1a3e4a7bca5886c206783ab7db8887a25af0012a`  Removed; provenance only |
| `src/ddt4all/core/elm/device_manager.py` | `6cebb40a43b860309d93c13527bc7eba25e1ad28`  Removed; provenance only |
| `src/ddt4all/core/doip/doip_message_type.py` | `c3ff60e77cf814e330ec60f467acaa1d397e1c5c`  Removed; provenance only |
| `src/ddt4all/core/doip/doip_protocol_error.py` | `622f6d78a3cbba7355e48a5f8fb2ec5091972d2c`  Removed; provenance only |
| `src/ddt4all/core/doip/doip_connection.py` | `b874e7391367302cffcda468bfaa5b1b8826c7fa`  Removed; provenance only |
| `tests/resources/chatgpt_generated/dummy_ecu_can.json` | `fa587681188cb8a374c402cc72e38be8808c97b9`  Retained |
| `tests/resources/chatgpt_generated/dummy_ecu_can.xml` | `d266452565dfcf953c3b4d59d72fa1b89e5d3edd`  Retained |
| `tests/resources/chatgpt_generated/dummy_ecu_can_ext.xml` | `64385fa97488a5ef4adc5625d9ab9cabac8fa179`  Retained |
| `tests/resources/chatgpt_generated/dummy_ecu_iso8.xml` | `3ce8c25d73a29680e79aa3bdde5e340397c153ae`  Retained |
| `tests/resources/chatgpt_generated/dummy_ecu_kwp.xml` | `32496c60e733b1cb54ef65180af21c7a16910316`  Retained |

## Database scope

The five copied XML/JSON files are upstream synthetic test fixtures, not a manufacturer ECU database. The actual ECU database is loaded externally by DDT4All from ecu.zip/db.json, XML or local JSON; ecu.zip is absent in the pinned tree. The public tree includes src/ddt4all/resources/projects.json (5,268,760 bytes, blob fdb1be422e26cd0c0b117da4db6125e049e25982), but its commit history includes https://github.com/cedricp/ddt4all/commit/fe3f97956cc4c7f3fd40925eca97291872a89e10 ("update project json data for uses recent ecus.zip"). Independent authorization for redistributing the underlying data has not been established; do not vendor or ship it until verified. The GitHub file retrieval also did not yield its contents in the inspected line range. Do not misrepresent synthetic fixtures as OEM coverage.

See [technical reference](../../DDT4ALL.md) and [roadmap](../../../ROADMAP.md). No claims of implementation, complete upstream mirroring or physical compatibility.

## Translation catalogs (data only)

All 16 files verified byte-for-byte against the pinned upstream tree. Original translator headers and license notices preserved. No compiled catalogs, gettext tooling, Python or Polib imported. Translation text must be mapped to OBDium keys and reviewed before use; original strings may contain product-specific names, HTML and Python-format placeholders. Do not execute catalog expressions or render untrusted markup. Source locale `pt` does not by itself certify PT-PT quality.

| Original path | Git blob SHA | Bytes |
|---|---|---:|
| `locales/cs_CZ/LC_MESSAGES/ddt4all.po` | `de6bd656e803dd4cfd3790621d1523ef9671cee4` | 64163 |
| `locales/ddt4all.pot` | `6e3da4ae9d76102f6b8d9a0b4451ed7853db3ef8` | 38272 |
| `locales/de/LC_MESSAGES/ddt4all.po` | `9ef9507c7e5c495679c36fdcca673c5b82a9087f` | 64602 |
| `locales/es/LC_MESSAGES/ddt4all.po` | `8f3435fc9cc7050f2e131e89ca2f9ef44868dad7` | 64343 |
| `locales/fr/LC_MESSAGES/ddt4all.po` | `a03c83c9556fbbc9cae2236d81284633b6946db9` | 65923 |
| `locales/hu/LC_MESSAGES/ddt4all.po` | `f87f177514df7432476f1fb3a8b87864abeaca4c` | 64636 |
| `locales/it/LC_MESSAGES/ddt4all.po` | `67afc329fb44e2ac0029db96843467e97c383db6` | 63602 |
| `locales/nl/LC_MESSAGES/ddt4all.po` | `50207cd2e06716d9964a24439cbd96f4cd65fef8` | 62651 |
| `locales/pl/LC_MESSAGES/ddt4all.po` | `1dd871856cf17d88abbde252975e6958b9325f09` | 64569 |
| `locales/pt/LC_MESSAGES/ddt4all.po` | `38750767aa74173db59baa71f0792dc2560ca3d6` | 64463 |
| `locales/ro/LC_MESSAGES/ddt4all.po` | `2ea91cc205ec1cd50a75bc20f727e50bd2769251` | 63934 |
| `locales/ru/LC_MESSAGES/ddt4all.po` | `4ff5af5b9cea73fc711bfa6d9234b72d13af627a` | 79104 |
| `locales/sr/LC_MESSAGES/ddt4all.po` | `c73e89dc8e7dbb116f8e94bec853e69b244bf394` | 63046 |
| `locales/tr/LC_MESSAGES/ddt4all.po` | `f3b56371faaf88fa41277217f33ee09a0da41ea4` | 63819 |
| `locales/uk_UA/LC_MESSAGES/ddt4all.po` | `ef3d039b55e39969734d90dfaae7b3c0887db1a4` | 78268 |
| `locales/zh_CN/LC_MESSAGES/ddt4all.po` | `df96d6d9b428bf2da9ac42590fa2e87df2c45b2c` | 60937 |
