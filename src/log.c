#include "log.h"

#include <windows.h>

#define QEMU_DEBUGCON_PORT 0xe9

static void debugcon_write (const char * str, int len);

void
iefixup_log (const char * format,
             ...)
{
  char message[1024];
  int prefix_len, len;
  va_list args;

  prefix_len = wsprintfA (message, "[iefixup %lx] ", GetCurrentThreadId ());

  va_start (args, format);
  len = prefix_len + wvsprintfA (message + prefix_len, format, args);
  va_end (args);

  debugcon_write (message, len);
}

static void
debugcon_write (const char * str,
                int len)
{
  for (; len != 0; len--, str++)
    __asm__ volatile ("outb %0, %1" : : "a" (*str), "Nd" ((unsigned short) QEMU_DEBUGCON_PORT));
}
