"""Offline repository integrity checks; no third-party runtime or network."""
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2] / "backend/data/catalogs"
ADMIN_FILES = {".gitattributes", "README.md", "manifest.json", "licenses/ATTRIBUTION.md"}
MAX_BYTES = 2 * 1024 * 1024
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


def verify_catalog(root):
    if root.is_symlink():
        raise ValueError("Catalog root must not be a symbolic link")
    manifest = json.loads(checked_file(root, "manifest.json"))
    if manifest["schema_version"] != 1 or manifest["status"] != "staged_not_enabled":
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
