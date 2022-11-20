#include <postgres.h>

#include <access/xact.h>
#include <access/xlog.h>
#include <bootstrap/bootstrap.h>
#include <miscadmin.h>
#include <postgres_ext.h>
#include <storage/ipc.h>
#include <storage/proc.h>
#include <storage/s_lock.h>
#include <utils/guc.h>
#include <utils/memutils.h>
#include <utils/pg_locale.h>
#include <utils/relmapper.h>

void pglite_set_bootstrap_processing_mode(void);
