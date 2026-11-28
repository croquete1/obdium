"""Offline repository integrity checks; no third-party runtime or network."""
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import tempfile
import unittest

REPO = Path(__file__).resolve().parents[2]
ROOT = REPO / "backend/data/catalogs"
VENDOR = REPO / "docs/references/vendor/ddt4all"
ADMIN_FILES = {".gitattributes", "README.md", "manifest.json", "licenses/ATTRIBUTION.md"}
MAX_BYTES = 2 * 1024 * 1024
PREVIEW_KEYS = {
    "confirm", "cancel", "error", "refresh", "stop", "close",
    "file", "exit", "help", "about", "no", "timeout",
}
SHA = re.compile(r"[0-9a-f]{40}")


def blob_sha(data):
    # Git SHA-1 object identity, not a cryptographic signature or rights proof.
    header = f"blob {len(data)}\0".encode("ascii")
    return hashlib.sha1(header + data, usedforsecurity=False).hexdigest()


def require_sha(value):
    if not isinstance(value, str) or SHA.fullmatch(value) is None:
        raise ValueError("Invalid Git object identity")


def checked_file(root, name):
    if not isinstance(name, str) or not name or "\\" in name or ":" in name:
        raise ValueError("Invalid catalog path")
    path = PurePosixPath(name)
    if path.is_absolute() or path.as_posix() != name or ".." in path.parts:
        raise ValueError("Invalid catalog path")
    current = root
    for part in path.parts:
        current = current / part
        if current.is_symlink():
            raise ValueError(f"Symbolic link in catalog: {name}")
    if not current.is_file():
        raise ValueError(f"Missing catalog file: {name}")
    with current.open("rb") as handle:
        data = handle.read(MAX_BYTES + 1)
    if len(data) > MAX_BYTES:
        raise ValueError(f"Catalog file too large: {name}")
    return data


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"Duplicate JSON key: {key}")
        result[key] = value
    return result


def reject_constant(value):
    raise ValueError(f"Invalid JSON constant: {value}")


def read_json(root, name):
    return json.loads(
        checked_file(root, name),
        object_pairs_hook=unique_object,
        parse_constant=reject_constant,
    )


def verify_catalog(root):
    if root.is_symlink():
        raise ValueError("Catalog root must not be a symbolic link")
    manifest = read_json(root, "manifest.json")
    if (type(manifest["schema_version"]) is not int
            or manifest["schema_version"] != 1
            or manifest["status"] != "staged_not_enabled"):
        raise ValueError("Unexpected manifest schema or activation status")
    expected = {}
    source_ids = set()

    def register(name, sha, size=None, shared=False):
        require_sha(sha)
        if name in ADMIN_FILES:
            raise ValueError(f"Unexpected data path: {name}")
        if name in expected:
            if not shared or expected[name] != sha:
                raise ValueError(f"Duplicate or conflicting catalog entry: {name}")
            return
        data = checked_file(root, name)
        if size is not None and (type(size) is not int or size != len(data)):
            raise ValueError(f"Byte length mismatch: {name}")
        if blob_sha(data) != sha:
            raise ValueError(f"Blob hash mismatch: {name}")
        expected[name] = sha

    for source in manifest["sources"]:
        source_id = source["id"]
        if not source_id or source_id in source_ids:
            raise ValueError("Duplicate or missing source ID")
        source_ids.add(source_id)
        require_sha(source["revision"])
        if not source["repository"] or not source["license"]:
            raise ValueError("Missing source attribution")
        for file in source.get("files", []):
            register(file["path"], file["git_blob_sha"], file["bytes"])
        if "derived_path" in source:
            require_sha(source["source_blob_sha"])
            register(source["derived_path"], source["derived_blob_sha"])
        if "license_path" in source:
            register(source["license_path"], source["license_blob_sha"], shared=True)
    actual = set()
    for path in root.rglob("*"):
        if path.is_symlink():
            raise ValueError("Unexpected symbolic link in catalog")
        if path.is_file():
            actual.add(path.relative_to(root).as_posix())
    if actual != set(expected) | ADMIN_FILES:
        raise ValueError("Catalog inventory differs from manifest")
    if not expected:
        raise ValueError("Empty catalog inventory")
    return len(expected)


def verify_vendor(root, revision):
    manifest = read_json(root, "manifest.json")
    if (type(manifest["schema_version"]) is not int
            or manifest["schema_version"] != 1
            or manifest["status"] != "reference_only"
            or manifest["repository"] != "cedricp/ddt4all"
            or manifest["revision"] != revision
            or manifest["license"] != "GPL-3.0-or-later"):
        raise ValueError("Unexpected vendor provenance")
    require_sha(revision)
    expected = {".gitattributes", "README.md", "manifest.json"}
    hashes = {}
    for entry in manifest["files"]:
        name = entry["path"]
        sha = entry["git_blob_sha"]
        require_sha(sha)
        if name in expected:
            raise ValueError(f"Duplicate vendor entry: {name}")
        data = checked_file(root, name)
        if "bytes" in entry and (
            type(entry["bytes"]) is not int or entry["bytes"] != len(data)
        ):
            raise ValueError(f"Vendor size mismatch: {name}")
        if blob_sha(data) != sha:
            raise ValueError(f"Vendor hash mismatch: {name}")
        expected.add(name)
        hashes[name] = sha
    actual = set()
    for path in root.rglob("*"):
        if path.is_symlink():
            raise ValueError("Unexpected vendor symbolic link")
        if path.is_file():
            actual.add(path.relative_to(root).as_posix())
    if actual != expected:
        raise ValueError("Vendor inventory differs from manifest")
    return hashes


def verify_translation_messages(document):
    if type(document["schema_version"]) is not int or document["schema_version"] != 1:
        raise ValueError("Unexpected translation schema")
    messages = document["messages"]
    if not isinstance(messages, dict) or set(messages) != PREVIEW_KEYS:
        raise ValueError("Unexpected translation keys")
    for message in messages.values():
        if (not isinstance(message, str) or not message.strip()
                or any(ord(ch) < 32 or 127 <= ord(ch) <= 159
                       or ch in "<>{}%" for ch in message)):
            raise ValueError("Invalid translation text")


def verify_translation_source(locale, document, hashes):
    if document["locale"] != locale or document["status"] != "draft_reference":
        raise ValueError("Unexpected translation identity or status")
    if locale == "en":
        if "source_blob_sha" in document:
            raise ValueError("English preview must remain independently authored")
        return
    original = {"cs-CZ": "cs_CZ", "pt-PT": "pt", "uk-UA": "uk_UA",
                "zh-CN": "zh_CN"}.get(locale, locale)
    path = f"locales/{original}/LC_MESSAGES/ddt4all.po"
    if path not in hashes or document.get("source_blob_sha") != hashes[path]:
        raise ValueError(f"Translation source mismatch: {locale}")


class VendorIntegrityTests(unittest.TestCase):
    def test_vendor_snapshot_and_translation_lineage(self):
        manifest = read_json(ROOT, "manifest.json")
        source = next(s for s in manifest["sources"] if s["id"] == "translation_preview")
        hashes = verify_vendor(VENDOR, source["revision"])
        self.assertEqual(len(hashes), 22)
        self.assertEqual(len(source["files"]), 16)
        for entry in source["files"]:
            locale = PurePosixPath(entry["path"]).stem
            document = read_json(ROOT, entry["path"])
            verify_translation_source(locale, document, hashes)
            verify_translation_messages(document)

    def test_translation_rejects_wrong_locale_source(self):
        hashes = {"locales/pt/LC_MESSAGES/ddt4all.po": "a" * 40,
                  "locales/fr/LC_MESSAGES/ddt4all.po": "b" * 40}
        document = {"locale": "pt-PT", "status": "draft_reference",
                    "source_blob_sha": "b" * 40}
        with self.assertRaisesRegex(ValueError, "Translation source mismatch"):
            verify_translation_source("pt-PT", document, hashes)

    def test_english_rejects_imported_source(self):
        document = {"locale": "en", "status": "draft_reference",
                    "source_blob_sha": "a" * 40}
        with self.assertRaisesRegex(ValueError, "independently authored"):
            verify_translation_source("en", document, {})

    def test_vendor_modified_file_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in (".gitattributes", "README.md"):
                (root / name).write_text("fixture", encoding="utf-8")
            (root / "license.txt").write_bytes(b"changed")
            revision = "a" * 40
            manifest = {
                "schema_version": 1, "status": "reference_only",
                "repository": "cedricp/ddt4all", "revision": revision,
                "license": "GPL-3.0-or-later",
                "files": [{"path": "license.txt", "git_blob_sha": blob_sha(b"original")}],
            }
            (root / "manifest.json").write_text(json.dumps(manifest), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "Vendor hash mismatch"):
                verify_vendor(root, revision)


class TranslationSchemaTests(unittest.TestCase):
    def setUp(self):
        self.document = {
            "schema_version": 1,
            "messages": {key: "Synthetic text" for key in PREVIEW_KEYS},
        }

    def test_exact_keys_are_valid(self):
        verify_translation_messages(self.document)

    def test_missing_key_rejected(self):
        self.document["messages"].pop("timeout")
        with self.assertRaisesRegex(ValueError, "translation keys"):
            verify_translation_messages(self.document)

    def test_extra_key_rejected(self):
        self.document["messages"]["unexpected"] = "Synthetic"
        with self.assertRaisesRegex(ValueError, "translation keys"):
            verify_translation_messages(self.document)

    def test_invalid_text_rejected(self):
        for value in ("", " ", "<b>text</b>", "{name}", "%s", "a\nb", None):
            with self.subTest(value=value):
                self.document["messages"]["confirm"] = value
                with self.assertRaisesRegex(ValueError, "translation text"):
                    verify_translation_messages(self.document)

    def test_boolean_schema_rejected(self):
        self.document["schema_version"] = True
        with self.assertRaisesRegex(ValueError, "translation schema"):
            verify_translation_messages(self.document)


class CatalogIntegrityTests(unittest.TestCase):
    def test_staged_catalog_inventory(self):
        self.assertEqual(verify_catalog(ROOT), 82)

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        for name in ADMIN_FILES - {"manifest.json"}:
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("administrative fixture", encoding="utf-8")
        self.payload = b"synthetic reference\n"
        (self.root / "data.json").write_bytes(self.payload)
        self.source = {
            "id": "synthetic",
            "revision": "a" * 40,
            "repository": "example/synthetic",
            "license": "MIT",
            "files": [{
                "path": "data.json",
                "git_blob_sha": blob_sha(self.payload),
                "bytes": len(self.payload),
            }],
        }
        self.manifest = {
            "schema_version": 1,
            "status": "staged_not_enabled",
            "sources": [self.source],
        }

    def verify(self):
        (self.root / "manifest.json").write_text(
            json.dumps(self.manifest), encoding="utf-8"
        )
        return verify_catalog(self.root)

    def test_valid_fixture(self):
        self.assertEqual(self.verify(), 1)

    def test_modified_same_length_file(self):
        (self.root / "data.json").write_bytes(b"x" * len(self.payload))
        with self.assertRaisesRegex(ValueError, "Blob hash mismatch"):
            self.verify()

    def test_missing_file(self):
        (self.root / "data.json").unlink()
        with self.assertRaisesRegex(ValueError, "Missing catalog file"):
            self.verify()

    def test_unlisted_file(self):
        (self.root / "unlisted.json").write_bytes(b"{}")
        with self.assertRaisesRegex(ValueError, "inventory differs"):
            self.verify()

    def test_traversal(self):
        self.source["files"][0]["path"] = "../data.json"
        with self.assertRaisesRegex(ValueError, "Invalid catalog path"):
            self.verify()

    def test_duplicate_path(self):
        self.source["files"].append(dict(self.source["files"][0]))
        with self.assertRaisesRegex(ValueError, "Duplicate or conflicting"):
            self.verify()

    def test_wrong_size(self):
        self.source["files"][0]["bytes"] += 1
        with self.assertRaisesRegex(ValueError, "Byte length mismatch"):
            self.verify()

    def test_invalid_revision(self):
        self.source["revision"] = "main"
        with self.assertRaisesRegex(ValueError, "Invalid Git object identity"):
            self.verify()

    def test_activation_not_allowed(self):
        self.manifest["status"] = "enabled"
        with self.assertRaisesRegex(ValueError, "activation status"):
            self.verify()

    def test_duplicate_json_field_rejected(self):
        with self.assertRaisesRegex(ValueError, "Duplicate JSON key"):
            (self.root / "manifest.json").write_text(
                '{"schema_version":1,"schema_version":1}', encoding="utf-8"
            )
            verify_catalog(self.root)

    def test_duplicate_nested_field_rejected(self):
        (self.root / "data.json").write_text(
            '{"messages":{"confirm":"first","confirm":"second"}}', encoding="utf-8"
        )
        with self.assertRaisesRegex(ValueError, "Duplicate JSON key"):
            read_json(self.root, "data.json")

    def test_nonstandard_json_constant_rejected(self):
        (self.root / "data.json").write_text('{"value":NaN}', encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "Invalid JSON constant"):
            read_json(self.root, "data.json")

    def test_boolean_manifest_schema_rejected(self):
        self.manifest["schema_version"] = True
        with self.assertRaisesRegex(ValueError, "manifest schema"):
            self.verify()
