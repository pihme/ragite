// TC-42 and TC-44. A Go program loads the extension built in this tree.
// The test opens that shared library directly. It does not require a published module.

package languages

import (
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
)

const document = "hello world"

func extensionPath() string {
	if raw := os.Getenv("RAGITE_EXTENSION"); raw != "" {
		return raw
	}
	return filepath.Join("..", "..", "..", "target", "debug", "libragite.so")
}

func TestTC42TC44CreateInsertMatch(t *testing.T) {
	mod := readGoMod(t)
	if strings.Contains(mod, "\nrequire") || strings.HasPrefix(mod, "require") {
		t.Fatal("TC-44: the Go test must not require a published module")
	}

	path := extensionPath()
	info, err := os.Stat(path)
	if err != nil || !info.Mode().IsRegular() {
		t.Fatalf("missing extension at %s", path)
	}
	if filepath.Base(path) != "libragite.so" {
		t.Fatalf("TC-44 loads the built extension, got %s", path)
	}

	db, err := openMemory()
	if err != nil {
		t.Fatal(err)
	}
	defer db.close()
	if err := db.loadExtension(path); err != nil {
		t.Fatal(err)
	}
	if err := db.exec("CREATE VIRTUAL TABLE docs USING ragite"); err != nil {
		t.Fatal(err)
	}
	if err := db.exec("INSERT INTO docs(text) VALUES ('hello world')"); err != nil {
		t.Fatal(err)
	}
	got, err := db.queryText("SELECT text FROM docs WHERE docs MATCH 'hello'")
	if err != nil {
		t.Fatal(err)
	}
	if got != document {
		t.Fatalf("MATCH returned %q, want %q", got, document)
	}
}

func readGoMod(t *testing.T) string {
	t.Helper()
	_, file, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("cannot locate the Go test file")
	}
	body, err := os.ReadFile(filepath.Join(filepath.Dir(file), "go.mod"))
	if err != nil {
		t.Fatalf("read go.mod: %v", err)
	}
	return string(body)
}
