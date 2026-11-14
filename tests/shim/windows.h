#ifndef __IEFIXUP_TEST_WINDOWS_H__
#define __IEFIXUP_TEST_WINDOWS_H__

#include <stdlib.h>

typedef int BOOL;
typedef unsigned short WORD;
typedef unsigned int DWORD;
typedef void * HANDLE;

#define TRUE 1
#define FALSE 0
#define HEAP_ZERO_MEMORY 8

#define GetProcessHeap() NULL
#define HeapAlloc(heap, flags, size) (((flags) & HEAP_ZERO_MEMORY) ? calloc (1, size) : malloc (size))
#define HeapReAlloc(heap, flags, p, size) realloc (p, size)
#define HeapFree(heap, flags, p) free (p)

#endif
