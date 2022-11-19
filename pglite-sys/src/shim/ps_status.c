#include "postgres.h"
#include "utils/ps_status.h"

bool __thread update_process_title = true;

char **
save_ps_display_args(int _argc, char **argv)
{
    return argv;
}

void
init_ps_display(const char *_fixed_part)
{
}

void
set_ps_display(const char *_activity)
{
}

const char *
get_ps_display(int *displen)
{
    *displen = 0;
    return NULL;
}
