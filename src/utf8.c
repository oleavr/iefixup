#include "utf8.h"

#define REPLACEMENT_CHARACTER '?'

static const WORD ascii_punctuation[][2] =
{
  { 0x2010, '-' }, { 0x2011, '-' }, { 0x2012, '-' }, { 0x2013, '-' }, { 0x2014, '-' }, { 0x2015, '-' },
  { 0x2018, '\'' }, { 0x2019, '\'' }, { 0x201a, '\'' }, { 0x201c, '"' }, { 0x201d, '"' }, { 0x201e, '"' },
  { 0x2022, '*' }, { 0x2026, '.' }, { 0x2039, '<' }, { 0x203a, '>' }, { 0x2190, '<' }, { 0x2192, '>' },
  { 0x2009, ' ' }, { 0x200a, ' ' }, { 0x202f, ' ' }, { 0x2212, '-' },
};

static const WORD cp1252_upper_half[32] =
{
  0x20ac, 0x0000, 0x201a, 0x0192, 0x201e, 0x2026, 0x2020, 0x2021,
  0x02c6, 0x2030, 0x0160, 0x2039, 0x0152, 0x0000, 0x017d, 0x0000,
  0x0000, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2013, 0x2014,
  0x02dc, 0x2122, 0x0161, 0x203a, 0x0153, 0x0000, 0x017e, 0x0178,
};

void
iefixup_utf8_decoder_init (IeFixupUtf8Decoder * self)
{
  self->codepoint = 0;
  self->pending = 0;
}

void
iefixup_utf8_decoder_feed (IeFixupUtf8Decoder * self,
                           unsigned char byte,
                           IeFixupBuffer * output)
{
  if ((byte & 0xc0) == 0x80)
  {
    if (self->pending == 0)
    {
      iefixup_buffer_append_byte (output, REPLACEMENT_CHARACTER);
      return;
    }
    self->codepoint = (self->codepoint << 6) | (byte & 0x3f);
    if (--self->pending == 0)
      iefixup_buffer_append_byte (output, iefixup_cp1252_from_codepoint (self->codepoint));
    return;
  }

  iefixup_utf8_decoder_flush (self, output);

  if (byte < 0x80)
  {
    iefixup_buffer_append_byte (output, byte);
  }
  else if ((byte & 0xe0) == 0xc0)
  {
    self->codepoint = byte & 0x1f;
    self->pending = 1;
  }
  else if ((byte & 0xf0) == 0xe0)
  {
    self->codepoint = byte & 0x0f;
    self->pending = 2;
  }
  else if ((byte & 0xf8) == 0xf0)
  {
    self->codepoint = byte & 0x07;
    self->pending = 3;
  }
  else
  {
    iefixup_buffer_append_byte (output, REPLACEMENT_CHARACTER);
  }
}

void
iefixup_utf8_decoder_flush (IeFixupUtf8Decoder * self,
                            IeFixupBuffer * output)
{
  if (self->pending != 0)
    iefixup_buffer_append_byte (output, REPLACEMENT_CHARACTER);
  self->pending = 0;
}

void
iefixup_utf8_append_as_cp1252 (IeFixupBuffer * output,
                               const unsigned char * data,
                               size_t size)
{
  IeFixupUtf8Decoder decoder;
  size_t i;

  iefixup_utf8_decoder_init (&decoder);
  for (i = 0; i != size; i++)
    iefixup_utf8_decoder_feed (&decoder, data[i], output);
  iefixup_utf8_decoder_flush (&decoder, output);
}

unsigned char
iefixup_cp1252_from_codepoint (DWORD codepoint)
{
  unsigned int i;

  if (codepoint < 0x80 || (codepoint >= 0xa0 && codepoint <= 0xff))
    return codepoint;

  for (i = 0; i != sizeof (ascii_punctuation) / sizeof (ascii_punctuation[0]); i++)
  {
    if (ascii_punctuation[i][0] == codepoint)
      return ascii_punctuation[i][1];
  }

  for (i = 0; i != 32; i++)
  {
    if (cp1252_upper_half[i] == codepoint)
      return 0x80 + i;
  }

  return REPLACEMENT_CHARACTER;
}
