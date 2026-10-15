"""TC-41 and TC-44. Python's sqlite3 loads the extension built in this tree.

The test opens that shared library directly. It does not install a package.
"""

import os
import sqlite3
import unittest
from pathlib import Path

TEXT = "hello world"
QUERY = "hello"


def extension_path():
    raw = os.environ.get("RAGITE_EXTENSION")
    if raw:
        return Path(raw)
    root = Path(__file__).resolve().parents[3]
    return root / "target" / "debug" / "libragite.so"


class LoadTest(unittest.TestCase):
    def test_tc_41_tc_44_create_insert_match(self):
        path = extension_path()
        self.assertTrue(path.is_file(), f"missing extension at {path}")
        self.assertEqual(path.name, "libragite.so")

        connection = sqlite3.connect(":memory:")
        self.addCleanup(connection.close)
        connection.enable_load_extension(True)
        connection.load_extension(os.fspath(path))
        connection.execute("CREATE VIRTUAL TABLE docs USING ragite")
        connection.execute("INSERT INTO docs(text) VALUES (?)", (TEXT,))
        rows = connection.execute(
            "SELECT text FROM docs WHERE docs MATCH ?",
            (QUERY,),
        ).fetchall()
        self.assertGreaterEqual(len(rows), 1)
        self.assertEqual(rows[0][0], TEXT)


if __name__ == "__main__":
    unittest.main()
