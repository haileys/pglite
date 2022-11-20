#include <stdio.h>

#include "postgres.h"
#include "libpq/pqsignal.h"

/* Global variables */
__thread sigset_t
UnBlockSig,
BlockSig,
StartupBlockSig;

void pqinitmask(void)
{
}

int pqsigsetmask(sigset_t mask)
{
    fprintf(stderr, "pqlite: pqsigsetmask: %x\n", mask);
}
