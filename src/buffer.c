#include "buffer.h"

#include <string.h>

static void iefixup_buffer_reserve (IeFixupBuffer * self, size_t extra);

void
iefixup_buffer_init (IeFixupBuffer * self)
{
  self->data = NULL;
  self->length = 0;
  self->capacity = 0;
}

void
iefixup_buffer_clear (IeFixupBuffer * self)
{
  if (self->data != NULL)
    HeapFree (GetProcessHeap (), 0, self->data);
  iefixup_buffer_init (self);
}

void
iefixup_buffer_append (IeFixupBuffer * self,
                       const void * data,
                       size_t size)
{
  iefixup_buffer_reserve (self, size);
  memcpy (self->data + self->length, data, size);
  self->length += size;
}

void
iefixup_buffer_append_byte (IeFixupBuffer * self,
                            unsigned char byte)
{
  iefixup_buffer_reserve (self, 1);
  self->data[self->length++] = byte;
}

void
iefixup_buffer_insert_repeated (IeFixupBuffer * self,
                                size_t offset,
                                unsigned char byte,
                                size_t count)
{
  iefixup_buffer_reserve (self, count);
  memmove (self->data + offset + count, self->data + offset, self->length - offset);
  memset (self->data + offset, byte, count);
  self->length += count;
}

void
iefixup_buffer_consume (IeFixupBuffer * self,
                        size_t size)
{
  memmove (self->data, self->data + size, self->length - size);
  self->length -= size;
}

static void
iefixup_buffer_reserve (IeFixupBuffer * self,
                        size_t extra)
{
  size_t needed = self->length + extra;

  if (needed <= self->capacity)
    return;

  self->capacity = (needed < 256) ? 256 : needed * 2;
  if (self->data == NULL)
    self->data = HeapAlloc (GetProcessHeap (), 0, self->capacity);
  else
    self->data = HeapReAlloc (GetProcessHeap (), 0, self->data, self->capacity);
}
