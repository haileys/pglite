#include <postgres.h>
#include <miscadmin.h>

void
pglite_set_bootstrap_processing_mode()
{
    SetProcessingMode(BootstrapProcessing);
}
