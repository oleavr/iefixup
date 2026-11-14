#include "stream.h"

static CRITICAL_SECTION streams_lock;
static IeFixupStream * streams = NULL;

void
iefixup_streams_init (void)
{
  InitializeCriticalSection (&streams_lock);
}

IeFixupStream *
iefixup_streams_find (void * handle)
{
  IeFixupStream * stream;

  EnterCriticalSection (&streams_lock);
  for (stream = streams; stream != NULL && stream->handle != handle; stream = stream->next)
    ;
  LeaveCriticalSection (&streams_lock);

  return stream;
}

IeFixupStream *
iefixup_streams_add (void * handle,
                     IeFixupHtml * html)
{
  IeFixupStream * stream;

  stream = HeapAlloc (GetProcessHeap (), HEAP_ZERO_MEMORY, sizeof (IeFixupStream));
  stream->handle = handle;
  stream->html = html;
  iefixup_buffer_init (&stream->pending);

  EnterCriticalSection (&streams_lock);
  stream->next = streams;
  streams = stream;
  LeaveCriticalSection (&streams_lock);

  return stream;
}

void
iefixup_streams_remove (void * handle)
{
  IeFixupStream ** link, * stream;

  EnterCriticalSection (&streams_lock);
  for (link = &streams; *link != NULL && (*link)->handle != handle; link = &(*link)->next)
    ;
  stream = *link;
  if (stream != NULL)
    *link = stream->next;
  LeaveCriticalSection (&streams_lock);

  if (stream == NULL)
    return;

  if (stream->html != NULL)
    iefixup_html_free (stream->html);
  iefixup_buffer_clear (&stream->pending);
  HeapFree (GetProcessHeap (), 0, stream);
}
