#include <postgres.h>

#include <fcntl.h>
#include <miscadmin.h>
#include <sys/stat.h>
#include <utils/elog.h>
#include <utils/palloc.h>

int
pglite_mkdir(const char* path, mode_t mode)
{
    int rc, save_errno;
    char* abs_path = psprintf("%s/%s", DataDir, path);

    elog(DEBUG1, "pglite_mkdir: %s\n", abs_path);
    errno = 0;
    rc = mkdir(abs_path, mode);

    save_errno = errno;
    pfree(abs_path);
    errno = save_errno;

    return rc;
}

DIR*
pglite_opendir(const char* path)
{
    DIR* dir;
    int save_errno;
    char* abs_path = psprintf("%s/%s", DataDir, path);

    elog(DEBUG1, "pglite_opendir: %s\n", abs_path);
    errno = 0;
    dir = opendir(abs_path);

    save_errno = errno;
    pfree(abs_path);
    errno = save_errno;

    return dir;
}

int
pglite_stat(const char* restrict path, struct stat* restrict buf)
{
    int rc, save_errno;
    char* abs_path = psprintf("%s/%s", DataDir, path);

    elog(DEBUG1, "pglite_stat: %s\n", abs_path);
    errno = 0;
    rc = stat(abs_path, buf);

    save_errno = errno;
    pfree(abs_path);
    errno = save_errno;

    return rc;
}

int
pglite_open(const char* path, int flags, mode_t mode)
{
    int fd, save_errno;
    char* abs_path = psprintf("%s/%s", DataDir, path);

    elog(DEBUG1, "pglite_open: %s\n", abs_path);
    errno = 0;
    fd = open(abs_path, flags, mode);

    save_errno = errno;
    pfree(abs_path);
    errno = save_errno;

    return fd;
}
