#include "sqlite3ext.h"
SQLITE_EXTENSION_INIT1
#include <assert.h>
#include <string.h>

int levenshtein(const char *s, const char *t) {
    int ls = strlen(s), lt = strlen(t);
    int d[ls + 1][lt + 1];

    for (int i = 0; i <= ls; i++)
        for (int j = 0; j <= lt; j++)
            d[i][j] = -1;

    int dist(int i, int j) {
        if (d[i][j] >= 0) return d[i][j];

        int x;
        if (i == ls)
            x = lt - j;
        else if (j == lt)
            x = ls - i;
        else if (s[i] == t[j])
            x = dist(i + 1, j + 1);
        else {
            x = dist(i + 1, j + 1);

            int y;
            if ((y = dist(i, j + 1)) < x) x = y;
            if ((y = dist(i + 1, j)) < x) x = y;
            x++;
        }
        return d[i][j] = x;
    }
    return dist(0, 0);
}

static void levenshtein_wrapper(
    sqlite3_context *context,
    int argc,
    sqlite3_value **argv
) {
    const char *a, *b;
    assert( argc==2 );
    if( sqlite3_value_type(argv[0])==SQLITE_NULL || sqlite3_value_type(argv[1])==SQLITE_NULL) return;
    a = sqlite3_value_text(argv[0]);
    b = sqlite3_value_text(argv[1]);
    sqlite3_result_int(context, levenshtein(a,b));
}

int sqlite3_levenshtein_init(
    sqlite3 *db,
    char **pzErrMsg,
    const sqlite3_api_routines *pApi) {
    int rc = SQLITE_OK;
    SQLITE_EXTENSION_INIT2(pApi);
    (void)pzErrMsg;  /* Unused parameter */
    rc = sqlite3_create_function(db, "levenshtein", 2, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL,
                                 levenshtein_wrapper, NULL, NULL);
    return rc;
}
