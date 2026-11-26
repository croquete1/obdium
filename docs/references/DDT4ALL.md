# DDT4All — technical integration reference for OBDium

Date: 2026-11-26. Source: [cedricp/ddt4all](https://github.com/cedricp/ddt4all/tree/09a39795725b7e510041436cdde3b1d65609142c), commit `09a39795725b7e510041436cdde3b1d65609142c`, README v3.1.6, GPL-3.0-or-later. OBDium baseline: `314fa378b4de35cd9c44a6bc2df2aab9dddefa11`. Canonical status and acceptance: [ROADMAP.md](../ROADMAP.md). This is an original technical synthesis, not a vendored implementation, OEM database, or proof of physical compatibility. DDT4All primarily covers DDT/Renault-family definitions; general multibrand coverage needs independent evidence. Existing OBDium PRs #4–#12 are active; do not duplicate their parsers or change their branches.

## Native Rust implementation constraint

All new diagnostic, transport, import, simulation, CLI and extension logic must be implemented in Rust. Do not embed, invoke, install or package Python, PyQt, pyserial, pyusb, polib or any other dependency taken from the reference project's stack. No FFI/sidecar/subprocess bridge to the reference application. Prefer existing Rust dependencies and standard library; any additional Rust crate requires a documented technical need, license and security review. Existing Tauri frontend and SQLite architecture remain unchanged. Existing CI tools are outside this integration; do not add Python tests or scripts for the new features. Port relevant tests into Rust and use inert XML/JSON fixtures. Preserve required provenance and notices for adapted code.

## Source map and portability decisions

| Reference (pinned) | Observed behavior / concept | OBDium work |
|---|---|---|
| [README](https://github.com/cedricp/ddt4all/blob/09a39795725b7e510041436cdde3b1d65609142c/README.md), [license](https://github.com/cedricp/ddt4all/blob/09a39795725b7e510041436cdde3b1d65609142c/license.txt) | Diagnostics, screens, logging, plugins, platforms, adapter matrix, limitations | F06–F12, F14–F21; preserve attribution |
| [ELM device manager](https://github.com/cedricp/ddt4all/blob/09a39795725b7e510041436cdde3b1d65609142c/src/ddt4all/core/elm/device_manager.py), [ELM](https://github.com/cedricp/ddt4all/blob/09a39795725b7e510041436cdde3b1d65609142c/src/ddt4all/core/elm/elm.py), [port](https://github.com/cedricp/ddt4all/blob/09a39795725b7e510041436cdde3b1d65609142c/src/ddt4all/core/elm/port.py), [constants](https://github.com/cedricp/ddt4all/blob/09a39795725b7e510041436cdde3b1d65609142c/src/ddt4all/core/elm/constants.py) | Device-specific configuration, port abstraction, ELM/STN negotiation | F03/F06; capability probing, not trusting firmware strings |
| [USB device](https://github.com/cedricp/ddt4all/blob/09a39795725b7e510041436cdde3b1d65609142c/src/ddt4all/core/usbdevice/obd_device.py), [USB CAN](https://github.com/cedricp/ddt4all/blob/09a39795725b7e510041436cdde3b1d65609142c/src/ddt4all/core/usbdevice/usb_can.py) | USB-specific transport support | F06/F19; use appropriate drivers |
| [ECU scanner](https://github.com/cedricp/ddt4all/blob/09a39795725b7e510041436cdde3b1d65609142c/src/ddt4all/core/ecu/ecu_scanner.py), [identification](https://github.com/cedricp/ddt4all/blob/09a39795725b7e510041436cdde3b1d65609142c/src/ddt4all/core/ecu/ecu_ident.py), [device](https://github.com/cedricp/ddt4all/blob/09a39795725b7e510041436cdde3b1d65609142c/src/ddt4all/core/ecu/ecu_device.py) | CAN/KWP/DoIP identification, matching diagnostic/software/supplier versions | F14; exact versus ambiguous matches, no blind scanning |
| [ECU database](https://github.com/cedricp/ddt4all/blob/09a39795725b7e510041436cdde3b1d65609142c/src/ddt4all/core/ecu/ecu_database.py), [ECU file](https://github.com/cedricp/ddt4all/blob/09a39795725b7e510041436cdde3b1d65609142c/src/ddt4all/core/ecu/ecu_file.py), [data](https://github.com/cedricp/ddt4all/blob/09a39795725b7e510041436cdde3b1d65609142c/src/ddt4all/core/ecu/ecu_data.py), [data item](https://github.com/cedricp/ddt4all/blob/09a39795725b7e510041436cdde3b1d65609142c/src/ddt4all/core/ecu/data_item.py), [request](https://github.com/cedricp/ddt4all/blob/09a39795725b7e510041436cdde3b1d65609142c/src/ddt4all/core/ecu/ecu_request.py) | XML/ZIP/JSON, field offsets, endianness, request and response definitions | F10/F15/F20; read-only normalized data model |
| [DoIP connection](https://github.com/cedricp/ddt4all/blob/09a39795725b7e510041436cdde3b1d65609142c/src/ddt4all/core/doip/doip_connection.py), [devices](https://github.com/cedricp/ddt4all/blob/09a39795725b7e510041436cdde3b1d65609142c/src/ddt4all/core/doip/doip_devices.py), [message types](https://github.com/cedricp/ddt4all/blob/09a39795725b7e510041436cdde3b1d65609142c/src/ddt4all/core/doip/doip_message_type.py) | Discovery, TCP framing, routing, alive checks, diagnostic messages | F17; mock peers, bounded framing and selected targets |
| [sniffer](https://github.com/cedricp/ddt4all/blob/09a39795725b7e510041436cdde3b1d65609142c/src/ddt4all/ui/sniffer/sniffer.py), [worker](https://github.com/cedricp/ddt4all/blob/09a39795725b7e510041436cdde3b1d65609142c/src/ddt4all/ui/sniffer/sniffer_thread.py) | Nonblocking CAN capture and filtering | F16/F08; passive capture only initially |
| [display widgets](https://github.com/cedricp/ddt4all/tree/09a39795725b7e510041436cdde3b1d65609142c/src/ddt4all/ui/displaymod), [data editor](https://github.com/cedricp/ddt4all/tree/09a39795725b7e510041436cdde3b1d65609142c/src/ddt4all/ui/data_editor) | User-defined screens, parameters and request editor | F07/F10; native Tauri UI, declarative layouts and validated read-only definitions |
| [plugins](https://github.com/cedricp/ddt4all/tree/09a39795725b7e510041436cdde3b1d65609142c/src/ddt4all/plugins), [CLI](https://github.com/cedricp/ddt4all/tree/09a39795725b7e510041436cdde3b1d65609142c/src/ddt4all/cli) | Vehicle-specific workflows, command macros and headless tools | F11/F12; restricted declarative workflows only, no arbitrary script execution |
| [tests](https://github.com/cedricp/ddt4all/tree/09a39795725b7e510041436cdde3b1d65609142c/tests), [CI](https://github.com/cedricp/ddt4all/blob/09a39795725b7e510041436cdde3b1d65609142c/.github/workflows/python-app.yml) | Unit/integration/smoke; synthetic ECU XML, mocked DoIP, legacy ISO8, USB | F02/F21; Rust fixtures, property tests and hardware evidence |
| [documentation](https://github.com/cedricp/ddt4all/tree/09a39795725b7e510041436cdde3b1d65609142c/documentation/ddt4all), [packaging](https://github.com/cedricp/ddt4all/tree/09a39795725b7e510041436cdde3b1d65609142c/setup_tools), [locales](https://github.com/cedricp/ddt4all/tree/09a39795725b7e510041436cdde3b1d65609142c/locales) | Protocol/module documentation, platform installers, community translations | F07/F12; preserve existing distribution and add verified compatibility guidance |

## Transport and protocol matrix

Keep host serial baud separate from vehicle bus bitrate and protocol initialization. ELM protocol selectors: 0 auto; 1 J1850 PWM 41.6 kbit/s; 2 VPW 10.4; 3 ISO 9141-2 (5-baud init, 10.4 kbit/s); 4 ISO 14230-4 slow init; 5 ISO 14230-4 fast init (both 10.4 kbit/s); 6/7 CAN 11/29-bit 500 kbit/s; 8/9 CAN 11/29-bit 250 kbit/s. PR #11/#12 cover negotiation/frame layouts; selector availability does not prove physical compatibility. ISO-TP classic CAN segmentation is a separate layer (PR #6/#7). UDS and KWP diagnostic services are separate from framing; DoIP is a separate transport. CAN FD, SAE J2534, OBDonUDS/J1979-2 and WWH-OBD/ISO 27145 are investigations, not features proven by DDT4All.

| Adapter family | Reference defaults (host-side, examples only) | Validation |
|---|---|---|
| ELM327 original/USB | 38400, 5 s | USB/serial and actual firmware |
| ELM clones | 9600–38400 | command support, response truncation, latency, false firmware claims |
| vLinker FS | 38400, 3 s; other speeds listed | exact revision/OS |
| Vgate iCar Pro | 115200, 2 s | identify variant and actual USB/SPP/BLE/TCP interface |
| OBDLink SX/EX | 115200, 2 s, RTS/CTS | negotiate supported rates and flow control |
| OBDLink MX+/CX | separately investigate SPP/BLE by model/platform | not claimed as DDT4All-tested |
| ELS27/V5 | 38400, 4 s reference | actual hardware, bus routing capabilities |
| DERLEK USB-DIAG | experimental, source README says real hardware not checked | no support claim |
| Native USB-CAN | driver-specific | receive-only capabilities, isolation and overflow |
| DoIP | selected Ethernet/IP interface | mock peer then real vehicle; not J2534 |

Values are source defaults to investigate, not settings to force. Use bounded handshake, manual override, negotiated capability flags, explicit unknown/unsupported, reconnect without replaying commands. Scan only selected/authorized interfaces. No automatic nonstandard pin routing on unknown hardware.

## Data and session contracts

Proposed names, not implementation: AdapterCapabilities, Transport, Protocol, SessionId, EcuAddress (CAN 11/29 bit, K-line logical, DoIP logical; protocol tagged), EcuIdentity (supplier/hardware/software/diagnostic version), DiagnosticRequest, DiagnosticResult, Measurement, CaptureFrame, DefinitionPack. Separate unverified inferred identity from exact response evidence. Keep raw response and interpreted value distinct; preserve source, unit, quality, ECU, timestamp and definition version. Use checked offsets, signed/unsigned widths, byte/bit order, bounded scaling, enum values and explicit missing/error. No arbitrary formula execution; reject non-finite values, overflow, duplicate keys and ambiguous overlaps. Untrusted packs cannot grant permissions or define executable code; all requests must pass the same backend allowlist and policy even if imported or from CLI.

XML/JSON/ZIP: parse locally, reject DTD/external entities, absolute/traversal/symlink archive paths, oversized files, compression bombs, duplicates, unknown schema versions and malformed inputs; limit count/depth/size before allocation. Normalize to canonical versioned model; transactional staging, preview, provenance/license, content hash, immutable versions, rollback; original file not modified. Unsupported constructs reported, never silently executed or treated as reads. Synthetic packs only for tests. No distribution of DDT2000/OEM databases or third-party graphics without explicit rights. Actual license of each imported pack is independent of software GPL.

## Execution groups and gates

| Group | Scope | Dependencies / evidence |
|---|---|---|
| A foundation | F01–F05, F02 regression simulator, all nine protocols, typed errors and DTC per ECU | Existing #4–#12, current exact HEAD checks; no parallel parser edits |
| B device/UX | F06/F07/F08/F09: device registry, TCP/SPP/BLE, editable screens, log export, workshop history | A contracts; hardware matrix, nonblocking UI, replay determinism |
| C declarative diagnostics | F10/F14/F15/F20: ECU identity, normalized profiles, secure import, local pack registry | A + capability checks; fixtures, unknown/ambiguous matching, malformed input tests |
| D capture/network | F16/F17: passive CAN, native DoIP | A + B transport contracts; read-only capture, mocked network peers and real hardware |
| E ecosystem | F12/F18/F19: CLI/plugins, manufacturer packs, CAN FD/J2534 research | C/D as applicable; per-brand legal sources, per-interface evidence |
| F operational validation | F11/F21: controlled procedures, safety review, HIL | A–E relevant scope; physical bench tests for active operations; fail closed |
| G optional mobile | F13 | stable desktop and proven platform transport |

Maximum three independent implementation fronts, no artificial PR count. Shared ROADMAP and critical transport/parser changes serialized per AGENTS.md. Claim roadmap ID, link PR, add regression before fix, independently review, check exact final SHA, preserve gates and record results. No automatic merge for this planning PR. If Issues disabled, use stable IDs and PR tracking until owner enables them.

## Feature acceptance notes

- ECU discovery: profile-specific safe read-only requests, bounded timeout/concurrency, cancellation, exact/ambiguous/unknown/no-response states; no identification from address alone; don't claim full vehicle coverage.
- Passive CAN: receive-only capability gate, bounded queues, dropped-frame counters, monotonic timestamp plus wall-clock metadata, filter/export/seek; no transmit/replay onto live vehicle; redact VIN and identifiers before sharing.
- DoIP: explicit selected gateway/network, validate version/inverse/header and payload limits, fragmented TCP reads, routing rejection, alive checks, timeouts, cancellation and negative diagnostic results; no broad probing, no security gateway bypass. Tests mock peers; real validation required.
- Custom screens: reusable read-only widgets, per-ECU data quality, layouts and screen profiles, stale/disconnected state, responsive accessible UI; no raw HTML or script import.
- CLI/plugins: versioned manifests, declarative read-only procedures, explicit budgets/permissions, no filesystem/network/hardware privileges beyond backend policy. Active operations separate F11 approval, documented conditions and validation; never import raw macros that can send writes.
- Brand coverage: Renault/Dacia pilot first if lawful definitions and fixtures available; VAG, Stellantis, BMW/MINI, Mercedes-Benz, Ford, Toyota/Lexus, Hyundai/Kia, Nissan, Volvo and others according to actual test availability; exact model/year/ECU/firmware and adapter support matrix rather than brand-level promises.
- HIL: selected USB ELM/STN, legacy K-line/J1850 where obtainable, CAN 11/29-bit and rates, Bluetooth, TCP, passive CAN and DoIP; record anonymized vehicle, OS, adapter firmware, build SHA, exact operation, expected/actual results and limitations. Bench simulator first, live vehicle reads only initially. No road testing of active operations.

## Licensing and exclusions

DDT4All is GPL-3.0-or-later; OBDium GPL-3.0. Reuse ideas/specifications and original Rust implementations first. For any adapted source, pin upstream commit/path, preserve notices and author credit, record modification and satisfy applicable GPL terms. Verify third-party licenses individually; never assume OEM data rights. No proprietary DDT archives, unsafe macros, airbag reset/coding, immobilizer programming, odometer manipulation, emissions defeat, safety deactivation, credential extraction or security bypass. No arbitrary flash/programming. Supported legitimate maintenance procedures require separate scope and validation. Source links provide reproducible reference, not a mirror of all upstream files.

## Status

Planning and source mapping only; no runtime feature or physical verification is claimed. Existing demo and OBD-II functions remain untouched. See canonical roadmap for individual TODO/BLOCKED/REVIEW statuses and evidence.

## Vendored reference files

Only the five synthetic XML/JSON fixtures, original GPL license and provenance manifest remain in [vendor/ddt4all](vendor/ddt4all/README.md). Previously copied Python modules were removed from the proposed tree at the maintainer's request; their original source remains accessible through the pinned links above. Actual OEM database and project catalog remain excluded pending data-specific rights verification; see manifest.
