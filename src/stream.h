#ifndef __IEFIXUP_STREAM_H__
#define __IEFIXUP_STREAM_H__

#include "buffer.h"
#include "html.h"

typedef struct _IeFixupStream IeFixupStream;

struct _IeFixupStream
{
  void * handle;
  IeFixupHtml * html;
  IeFixupBuffer pending;
  BOOL finished;
  IeFixupStream * next;
};

void iefixup_streams_init (void);
IeFixupStream * iefixup_streams_find (void * handle);
IeFixupStream * iefixup_streams_add (void * handle, IeFixupHtml * html);
void iefixup_streams_remove (void * handle);

#endif
