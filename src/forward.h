#ifndef __IEFIXUP_FORWARD_H__
#define __IEFIXUP_FORWARD_H__

#include <windows.h>

extern const char * const iefixup_forward_names[];
extern void * iefixup_forward_targets[];
extern const unsigned int iefixup_n_forwards;

void iefixup_forward_init (HMODULE real);

#endif
