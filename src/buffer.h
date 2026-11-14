#ifndef __IEFIXUP_BUFFER_H__
#define __IEFIXUP_BUFFER_H__

#include <windows.h>

typedef struct _IeFixupBuffer IeFixupBuffer;

struct _IeFixupBuffer
{
  unsigned char * data;
  size_t length;
  size_t capacity;
};

void iefixup_buffer_init (IeFixupBuffer * self);
void iefixup_buffer_clear (IeFixupBuffer * self);

void iefixup_buffer_append (IeFixupBuffer * self, const void * data, size_t size);
void iefixup_buffer_append_byte (IeFixupBuffer * self, unsigned char byte);
void iefixup_buffer_insert_repeated (IeFixupBuffer * self, size_t offset, unsigned char byte, size_t count);
void iefixup_buffer_consume (IeFixupBuffer * self, size_t size);

#endif
