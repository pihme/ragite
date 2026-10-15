// TC-43 and TC-44. node:sqlite loads the extension built in this tree.
// The test opens that shared library directly. It does not install a package.

import assert from 'node:assert/strict';
import { existsSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { DatabaseSync } from 'node:sqlite';

const TEXT = 'hello world';
const QUERY = 'hello';

function extensionPath() {
  if (process.env.RAGITE_EXTENSION) {
    return process.env.RAGITE_EXTENSION;
  }
  const here = dirname(fileURLToPath(import.meta.url));
  return resolve(here, '../../../target/debug/libragite.so');
}

test('TC-43 and TC-44: create, insert, and MATCH', () => {
  const path = extensionPath();
  assert.equal(existsSync(path), true, `missing extension at ${path}`);
  assert.equal(path.endsWith('libragite.so'), true, path);

  const db = new DatabaseSync(':memory:', { allowExtension: true });
  try {
    db.loadExtension(path);
    db.exec('CREATE VIRTUAL TABLE docs USING ragite');
    db.prepare('INSERT INTO docs(text) VALUES (?)').run(TEXT);
    const rows = db.prepare('SELECT text FROM docs WHERE docs MATCH ?').all(QUERY);
    assert.ok(rows.length >= 1);
    assert.equal(rows[0].text, TEXT);
  } finally {
    db.close();
  }
});
