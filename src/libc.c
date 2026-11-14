#include <stddef.h>

void *
memcpy (void * dst,
        const void * src,
        size_t n)
{
  unsigned char * d = dst;
  const unsigned char * s = src;

  while (n-- != 0)
    *d++ = *s++;

  return dst;
}

void *
memmove (void * dst,
         const void * src,
         size_t n)
{
  unsigned char * d = dst;
  const unsigned char * s = src;

  if (d < s)
  {
    while (n-- != 0)
      *d++ = *s++;
  }
  else
  {
    d += n;
    s += n;
    while (n-- != 0)
      *--d = *--s;
  }

  return dst;
}

void *
memset (void * dst,
        int c,
        size_t n)
{
  unsigned char * d = dst;

  while (n-- != 0)
    *d++ = (unsigned char) c;

  return dst;
}

int
memcmp (const void * a,
        const void * b,
        size_t n)
{
  const unsigned char * x = a;
  const unsigned char * y = b;

  for (; n != 0; n--, x++, y++)
  {
    if (*x != *y)
      return *x - *y;
  }

  return 0;
}

size_t
strlen (const char * str)
{
  const char * end;

  for (end = str; *end != '\0'; end++)
    ;

  return end - str;
}

int
strcmp (const char * a,
        const char * b)
{
  for (; *a != '\0' && *a == *b; a++, b++)
    ;

  return (unsigned char) *a - (unsigned char) *b;
}

char *
strcpy (char * dst,
        const char * src)
{
  char * d = dst;

  while ((*d++ = *src++) != '\0')
    ;

  return dst;
}
