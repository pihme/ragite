/* sqlite3_vtab_in is newer than the libsqlite3-sys loadable bindings.
   The host that loads this extension is the SQLite named in third_party/sqlite. */
#include "sqlite3ext.h"

extern const sqlite3_api_routines *sqlite3_api;

int ragite_vtab_in(sqlite3_index_info *info, int iCons, int bHandle) {
  if (sqlite3_api == 0 || sqlite3_api->vtab_in == 0) {
    return 0;
  }
  return sqlite3_api->vtab_in(info, iCons, bHandle);
}

int ragite_vtab_in_first(sqlite3_value *value, sqlite3_value **out) {
  if (sqlite3_api == 0 || sqlite3_api->vtab_in_first == 0) {
    return 1; /* SQLITE_ERROR */
  }
  return sqlite3_api->vtab_in_first(value, out);
}

int ragite_vtab_in_next(sqlite3_value *value, sqlite3_value **out) {
  if (sqlite3_api == 0 || sqlite3_api->vtab_in_next == 0) {
    return 1; /* SQLITE_ERROR */
  }
  return sqlite3_api->vtab_in_next(value, out);
}
