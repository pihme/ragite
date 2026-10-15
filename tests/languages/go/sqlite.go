package languages

/*
#cgo LDFLAGS: -l:libsqlite3.so.0

// The host package ships libsqlite3.so.0. The unversioned libsqlite3.so
// linker name is not installed with that package.
#include <stdlib.h>

#define SQLITE_OK 0
#define SQLITE_ROW 100
#define SQLITE_DONE 101

typedef struct sqlite3 sqlite3;
typedef struct sqlite3_stmt sqlite3_stmt;

int sqlite3_open(const char *filename, sqlite3 **ppDb);
int sqlite3_close(sqlite3 *);
int sqlite3_enable_load_extension(sqlite3 *db, int onoff);
int sqlite3_load_extension(
    sqlite3 *db, const char *zFile, const char *zProc, char **pzErrMsg);
int sqlite3_prepare_v2(
    sqlite3 *db, const char *zSql, int nByte, sqlite3_stmt **ppStmt,
    const char **pzTail);
int sqlite3_step(sqlite3_stmt *);
int sqlite3_finalize(sqlite3_stmt *);
const unsigned char *sqlite3_column_text(sqlite3_stmt *, int iCol);
const char *sqlite3_errmsg(sqlite3 *);
void sqlite3_free(void *);
*/
import "C"

import (
	"fmt"
	"unsafe"
)

type conn struct {
	db *C.sqlite3
}

func openMemory() (*conn, error) {
	cname := C.CString(":memory:")
	defer C.free(unsafe.Pointer(cname))
	var db *C.sqlite3
	rc := C.sqlite3_open(cname, &db)
	if rc != C.SQLITE_OK {
		msg := ""
		if db != nil {
			msg = C.GoString(C.sqlite3_errmsg(db))
			C.sqlite3_close(db)
		}
		return nil, fmt.Errorf("sqlite3_open: %s (%d)", msg, int(rc))
	}
	return &conn{db: db}, nil
}

func (c *conn) close() {
	if c.db != nil {
		C.sqlite3_close(c.db)
		c.db = nil
	}
}

func (c *conn) loadExtension(path string) error {
	if rc := C.sqlite3_enable_load_extension(c.db, 1); rc != C.SQLITE_OK {
		return fmt.Errorf("enable_load_extension: %s", C.GoString(C.sqlite3_errmsg(c.db)))
	}
	cpath := C.CString(path)
	defer C.free(unsafe.Pointer(cpath))
	var errMsg *C.char
	rc := C.sqlite3_load_extension(c.db, cpath, nil, &errMsg)
	if rc != C.SQLITE_OK {
		detail := ""
		if errMsg != nil {
			detail = C.GoString(errMsg)
			C.sqlite3_free(unsafe.Pointer(errMsg))
		}
		return fmt.Errorf("load_extension: %s (%s)", detail, C.GoString(C.sqlite3_errmsg(c.db)))
	}
	return nil
}

func (c *conn) exec(sql string) error {
	stmt, err := c.prepare(sql)
	if err != nil {
		return err
	}
	defer C.sqlite3_finalize(stmt)
	if rc := C.sqlite3_step(stmt); rc != C.SQLITE_DONE {
		return fmt.Errorf("step %q: %s (%d)", sql, C.GoString(C.sqlite3_errmsg(c.db)), int(rc))
	}
	return nil
}

func (c *conn) queryText(sql string) (string, error) {
	stmt, err := c.prepare(sql)
	if err != nil {
		return "", err
	}
	defer C.sqlite3_finalize(stmt)
	rc := C.sqlite3_step(stmt)
	if rc != C.SQLITE_ROW {
		return "", fmt.Errorf("no row (%d): %s", int(rc), C.GoString(C.sqlite3_errmsg(c.db)))
	}
	raw := C.sqlite3_column_text(stmt, 0)
	if raw == nil {
		return "", fmt.Errorf("NULL text")
	}
	return C.GoString((*C.char)(unsafe.Pointer(raw))), nil
}

func (c *conn) prepare(sql string) (*C.sqlite3_stmt, error) {
	csql := C.CString(sql)
	defer C.free(unsafe.Pointer(csql))
	var stmt *C.sqlite3_stmt
	var tail *C.char
	rc := C.sqlite3_prepare_v2(c.db, csql, C.int(-1), &stmt, &tail)
	if rc != C.SQLITE_OK {
		return nil, fmt.Errorf("prepare %q: %s", sql, C.GoString(C.sqlite3_errmsg(c.db)))
	}
	return stmt, nil
}
