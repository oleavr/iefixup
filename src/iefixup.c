#include "forward.h"
#include "html.h"
#include "log.h"
#include "stream.h"

#include <string.h>
#include <windows.h>

#define HTTP_QUERY_CONTENT_TYPE 1

typedef void * HINTERNET;

static void load_real_wininet (void);
static IeFixupStream * stream_for_handle (HINTERNET handle);
static BOOL is_html_response (HINTERNET handle);
static void stream_feed (IeFixupStream * stream, const unsigned char * data, DWORD size);
static size_t find_padding_offset (const IeFixupBuffer * output, size_t start);

static BOOL (WINAPI * real_InternetQueryDataAvailable) (HINTERNET file, DWORD * available, DWORD flags,
    DWORD context);
static BOOL (WINAPI * real_InternetReadFile) (HINTERNET file, void * buffer, DWORD size, DWORD * size_read);
static BOOL (WINAPI * real_InternetCloseHandle) (HINTERNET handle);
static BOOL (WINAPI * real_HttpQueryInfoA) (HINTERNET request, DWORD info_level, void * buffer,
    DWORD * buffer_size, DWORD * index);

BOOL WINAPI
DllMain (HINSTANCE instance,
         DWORD reason,
         void * reserved)
{
  if (reason == DLL_PROCESS_ATTACH)
  {
    iefixup_streams_init ();
    load_real_wininet ();
  }

  return TRUE;
}

BOOL WINAPI
InternetQueryDataAvailable (HINTERNET file,
                            DWORD * available,
                            DWORD flags,
                            DWORD context)
{
  IeFixupStream * stream;

  stream = stream_for_handle (file);
  if (stream->html == NULL || (stream->pending.length == 0 && !stream->finished))
    return real_InternetQueryDataAvailable (file, available, flags, context);

  *available = stream->pending.length;

  return TRUE;
}

BOOL WINAPI
InternetReadFile (HINTERNET file,
                  void * buffer,
                  DWORD size,
                  DWORD * size_read)
{
  IeFixupStream * stream;
  DWORD n;

  stream = stream_for_handle (file);
  if (stream->html == NULL)
    return real_InternetReadFile (file, buffer, size, size_read);

  if (stream->pending.length == 0 && !stream->finished)
  {
    if (!real_InternetReadFile (file, buffer, size, &n))
      return FALSE;
    stream_feed (stream, buffer, n);
  }

  n = (stream->pending.length < size) ? stream->pending.length : size;
  memcpy (buffer, stream->pending.data, n);
  iefixup_buffer_consume (&stream->pending, n);
  *size_read = n;

  return TRUE;
}

BOOL WINAPI
InternetCloseHandle (HINTERNET handle)
{
  iefixup_streams_remove (handle);

  return real_InternetCloseHandle (handle);
}

static void
load_real_wininet (void)
{
  char path[MAX_PATH];
  HMODULE real;

  GetSystemDirectoryA (path, MAX_PATH);
  lstrcatA (path, "\\WININET.DLL");
  real = LoadLibraryA (path);

  iefixup_forward_init (real);

  real_InternetQueryDataAvailable = (void *) GetProcAddress (real, "InternetQueryDataAvailable");
  real_InternetReadFile = (void *) GetProcAddress (real, "InternetReadFile");
  real_InternetCloseHandle = (void *) GetProcAddress (real, "InternetCloseHandle");
  real_HttpQueryInfoA = (void *) GetProcAddress (real, "HttpQueryInfoA");
}

static IeFixupStream *
stream_for_handle (HINTERNET handle)
{
  IeFixupStream * stream;

  stream = iefixup_streams_find (handle);
  if (stream == NULL)
    stream = iefixup_streams_add (handle, is_html_response (handle) ? iefixup_html_new () : NULL);

  return stream;
}

static BOOL
is_html_response (HINTERNET handle)
{
  char content_type[128];
  DWORD size = sizeof (content_type);
  DWORD error;
  BOOL is_html;

  error = GetLastError ();
  is_html = real_HttpQueryInfoA (handle, HTTP_QUERY_CONTENT_TYPE, content_type, &size, NULL) && size >= 9 &&
      CompareStringA (LOCALE_SYSTEM_DEFAULT, NORM_IGNORECASE, content_type, 9, "text/html", 9) == CSTR_EQUAL;
  SetLastError (error);

  IEFIXUP_LOG ("stream %08lx: %s\n", (DWORD) handle, is_html ? "rewriting HTML" : "passing through");

  return is_html;
}

static void
stream_feed (IeFixupStream * stream,
             const unsigned char * data,
             DWORD size)
{
  size_t start, produced;

  if (size == 0)
  {
    iefixup_html_finish (stream->html, &stream->pending);
    stream->finished = TRUE;
    return;
  }

  start = stream->pending.length;
  iefixup_html_feed (stream->html, data, size, &stream->pending);

  produced = stream->pending.length - start;
  if (produced < size)
    iefixup_buffer_insert_repeated (&stream->pending, find_padding_offset (&stream->pending, start), ' ',
        size - produced);
}

static size_t
find_padding_offset (const IeFixupBuffer * output,
                     size_t start)
{
  size_t i;

  for (i = output->length; i != start; i--)
  {
    if (output->data[i - 1] == '>')
      return i - 1;
  }

  for (i = output->length; i != start; i--)
  {
    if (output->data[i - 1] == ' ' || output->data[i - 1] == '\n')
      return i;
  }

  return output->length;
}
