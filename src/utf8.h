#ifndef __IEFIXUP_UTF8_H__
#define __IEFIXUP_UTF8_H__

#include "buffer.h"

typedef struct _IeFixupUtf8Decoder IeFixupUtf8Decoder;

struct _IeFixupUtf8Decoder
{
  DWORD codepoint;
  unsigned int pending;
};

void iefixup_utf8_decoder_init (IeFixupUtf8Decoder * self);
void iefixup_utf8_decoder_feed (IeFixupUtf8Decoder * self, unsigned char byte, IeFixupBuffer * output);
void iefixup_utf8_decoder_flush (IeFixupUtf8Decoder * self, IeFixupBuffer * output);

void iefixup_utf8_append_as_cp1252 (IeFixupBuffer * output, const unsigned char * data, size_t size);
unsigned char iefixup_cp1252_from_codepoint (DWORD codepoint);

#endif
