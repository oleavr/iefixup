#include "html.h"

#include "utf8.h"

#include <string.h>

#define MAX_NAME_LENGTH 15
#define MAX_TAG_LENGTH 65536

typedef enum _IeFixupHtmlState IeFixupHtmlState;
typedef struct _IeFixupTag IeFixupTag;
typedef struct _IeFixupAttribute IeFixupAttribute;

enum _IeFixupHtmlState
{
  IEFIXUP_HTML_TEXT,
  IEFIXUP_HTML_TAG,
  IEFIXUP_HTML_COMMENT,
};

struct _IeFixupHtml
{
  IeFixupHtmlState state;
  IeFixupUtf8Decoder text_decoder;
  IeFixupBuffer tag;
  unsigned char quote;
  unsigned int comment_dashes;
  char suppressed_element[MAX_NAME_LENGTH + 1];
};

struct _IeFixupTag
{
  const unsigned char * data;
  size_t length;
  BOOL closing;
  BOOL self_closing;
  char name[MAX_NAME_LENGTH + 1];
  size_t attributes_offset;
};

struct _IeFixupAttribute
{
  char name[MAX_NAME_LENGTH + 1];
  const unsigned char * raw;
  size_t raw_length;
  const unsigned char * value;
  size_t value_length;
};

static void iefixup_html_process_byte (IeFixupHtml * self, unsigned char byte, IeFixupBuffer * output);
static void iefixup_html_process_text_byte (IeFixupHtml * self, unsigned char byte, IeFixupBuffer * output);
static void iefixup_html_begin_tag (IeFixupHtml * self, IeFixupBuffer * output);
static void iefixup_html_process_tag_byte (IeFixupHtml * self, unsigned char byte, IeFixupBuffer * output);
static void iefixup_html_process_suppressed_tag_byte (IeFixupHtml * self, unsigned char byte,
    IeFixupBuffer * output);
static void iefixup_html_process_comment_byte (IeFixupHtml * self, unsigned char byte);
static void iefixup_html_flush_tag (IeFixupHtml * self, IeFixupBuffer * output);
static void iefixup_html_end_tag (IeFixupHtml * self, IeFixupBuffer * output);
static void iefixup_html_rewrite_tag (IeFixupHtml * self, const IeFixupTag * tag, IeFixupBuffer * output);

static void iefixup_tag_parse (IeFixupTag * self, const unsigned char * data, size_t length);
static BOOL iefixup_tag_find_attribute (const IeFixupTag * self, const char * name, IeFixupAttribute * attribute);
static BOOL iefixup_tag_next_attribute (const IeFixupTag * self, size_t * offset, IeFixupAttribute * attribute);
static void iefixup_tag_append_image_replacement (const IeFixupTag * self, IeFixupBuffer * output);
static void iefixup_tag_append_attributes (const IeFixupTag * self, IeFixupBuffer * output);

static BOOL is_svg_url (const unsigned char * url, size_t length);
static size_t read_name (const unsigned char * data, size_t length, size_t offset, char * name);
static BOOL is_name_char (unsigned char c);
static BOOL is_space (unsigned char c);
static BOOL is_listed (const char * name, const char * const * list);

static const char * const suppressed_elements[] =
{
  "script", "style", "template", "svg", "iframe", "object", "canvas", "video", "audio", NULL
};

static const char * const dropped_elements[] =
{
  "link", "noscript", "picture", "source", "track", "wbr", NULL
};

static const char * const block_elements[] =
{
  "header", "nav", "main", "section", "footer", "article", "aside", "figure", "figcaption",
  "hgroup", "details", "summary", "dialog", "search", NULL
};

static const char * const allowed_attributes[] =
{
  "href", "src", "alt", "name", "title", "width", "height", "border", "align", "valign", "colspan",
  "rowspan", "nowrap", "cellpadding", "cellspacing", "bgcolor", "color", "face", "size", "clear",
  "hspace", "vspace", "usemap", "shape", "coords", "target", "action", "method", "type", "value",
  "maxlength", "checked", "selected", "multiple", "cols", "rows", "start", "content", "http-equiv",
  NULL
};

IeFixupHtml *
iefixup_html_new (void)
{
  IeFixupHtml * self;

  self = HeapAlloc (GetProcessHeap (), HEAP_ZERO_MEMORY, sizeof (IeFixupHtml));

  self->state = IEFIXUP_HTML_TEXT;
  iefixup_utf8_decoder_init (&self->text_decoder);
  iefixup_buffer_init (&self->tag);

  return self;
}

void
iefixup_html_free (IeFixupHtml * self)
{
  iefixup_buffer_clear (&self->tag);
  HeapFree (GetProcessHeap (), 0, self);
}

void
iefixup_html_feed (IeFixupHtml * self,
                   const unsigned char * data,
                   size_t size,
                   IeFixupBuffer * output)
{
  size_t i;

  for (i = 0; i != size; i++)
    iefixup_html_process_byte (self, data[i], output);
}

void
iefixup_html_finish (IeFixupHtml * self,
                     IeFixupBuffer * output)
{
  if (self->state == IEFIXUP_HTML_TAG)
    iefixup_html_flush_tag (self, output);
  iefixup_utf8_decoder_flush (&self->text_decoder, output);
}

static void
iefixup_html_process_byte (IeFixupHtml * self,
                           unsigned char byte,
                           IeFixupBuffer * output)
{
  switch (self->state)
  {
    case IEFIXUP_HTML_TEXT:
      iefixup_html_process_text_byte (self, byte, output);
      break;
    case IEFIXUP_HTML_TAG:
      if (self->suppressed_element[0] != '\0')
        iefixup_html_process_suppressed_tag_byte (self, byte, output);
      else
        iefixup_html_process_tag_byte (self, byte, output);
      break;
    case IEFIXUP_HTML_COMMENT:
      iefixup_html_process_comment_byte (self, byte);
      break;
  }
}

static void
iefixup_html_process_text_byte (IeFixupHtml * self,
                                unsigned char byte,
                                IeFixupBuffer * output)
{
  if (byte == '<')
    iefixup_html_begin_tag (self, output);
  else if (self->suppressed_element[0] == '\0')
    iefixup_utf8_decoder_feed (&self->text_decoder, byte, output);
}

static void
iefixup_html_begin_tag (IeFixupHtml * self,
                        IeFixupBuffer * output)
{
  iefixup_utf8_decoder_flush (&self->text_decoder, output);

  self->state = IEFIXUP_HTML_TAG;
  self->tag.length = 0;
  self->quote = 0;
  iefixup_buffer_append_byte (&self->tag, '<');
}

static void
iefixup_html_process_tag_byte (IeFixupHtml * self,
                               unsigned char byte,
                               IeFixupBuffer * output)
{
  unsigned char previous = self->tag.data[self->tag.length - 1];

  if (self->tag.length == 1 && !(is_name_char (byte) || byte == '/' || byte == '!' || byte == '?'))
  {
    self->state = IEFIXUP_HTML_TEXT;
    iefixup_buffer_append_byte (output, '<');
    iefixup_html_process_text_byte (self, byte, output);
    return;
  }

  iefixup_buffer_append_byte (&self->tag, byte);

  if (self->tag.length == 4 && memcmp (self->tag.data, "<!--", 4) == 0)
  {
    self->state = IEFIXUP_HTML_COMMENT;
    self->comment_dashes = 0;
    return;
  }

  if (self->quote != 0)
  {
    if (byte == self->quote)
      self->quote = 0;
  }
  else if ((byte == '"' || byte == '\'') && previous == '=')
  {
    self->quote = byte;
  }
  else if (byte == '>')
  {
    iefixup_html_end_tag (self, output);
    return;
  }

  if (self->tag.length == MAX_TAG_LENGTH)
    iefixup_html_flush_tag (self, output);
}

static void
iefixup_html_process_suppressed_tag_byte (IeFixupHtml * self,
                                          unsigned char byte,
                                          IeFixupBuffer * output)
{
  if (byte == '<')
  {
    iefixup_html_begin_tag (self, output);
    return;
  }

  iefixup_buffer_append_byte (&self->tag, byte);

  if (byte == '>')
    iefixup_html_end_tag (self, output);
  else if (self->tag.length == MAX_TAG_LENGTH)
    self->state = IEFIXUP_HTML_TEXT;
}

static void
iefixup_html_process_comment_byte (IeFixupHtml * self,
                                   unsigned char byte)
{
  if (byte == '>' && self->comment_dashes >= 2)
    self->state = IEFIXUP_HTML_TEXT;

  self->comment_dashes = (byte == '-') ? self->comment_dashes + 1 : 0;
}

static void
iefixup_html_flush_tag (IeFixupHtml * self,
                        IeFixupBuffer * output)
{
  self->state = IEFIXUP_HTML_TEXT;

  if (self->suppressed_element[0] == '\0')
    iefixup_utf8_append_as_cp1252 (output, self->tag.data, self->tag.length);
}

static void
iefixup_html_end_tag (IeFixupHtml * self,
                      IeFixupBuffer * output)
{
  IeFixupTag tag;

  self->state = IEFIXUP_HTML_TEXT;

  iefixup_tag_parse (&tag, self->tag.data, self->tag.length);
  iefixup_html_rewrite_tag (self, &tag, output);
}

static void
iefixup_html_rewrite_tag (IeFixupHtml * self,
                          const IeFixupTag * tag,
                          IeFixupBuffer * output)
{
  const char * name = tag->name;

  if (self->suppressed_element[0] != '\0')
  {
    if (tag->closing && strcmp (name, self->suppressed_element) == 0)
      self->suppressed_element[0] = '\0';
    return;
  }

  if (name[0] == '\0' || is_listed (name, dropped_elements))
    return;

  if (is_listed (name, suppressed_elements))
  {
    if (!tag->closing && !tag->self_closing)
      strcpy (self->suppressed_element, name);
    return;
  }

  if (strcmp (name, "img") == 0)
  {
    iefixup_tag_append_image_replacement (tag, output);
    return;
  }

  if (is_listed (name, block_elements))
    name = "div";

  iefixup_buffer_append_byte (output, '<');
  if (tag->closing)
    iefixup_buffer_append_byte (output, '/');
  iefixup_buffer_append (output, name, strlen (name));
  if (!tag->closing)
    iefixup_tag_append_attributes (tag, output);
  iefixup_buffer_append_byte (output, '>');

  if (tag->closing && strcmp (name, "span") == 0)
    iefixup_buffer_append_byte (output, ' ');
}

static void
iefixup_tag_parse (IeFixupTag * self,
                   const unsigned char * data,
                   size_t length)
{
  size_t offset = 1;

  self->data = data;
  self->length = length;

  self->closing = data[offset] == '/';
  if (self->closing)
    offset++;

  self->attributes_offset = read_name (data, length, offset, self->name);
  self->self_closing = length >= 3 && data[length - 2] == '/';
}

static BOOL
iefixup_tag_find_attribute (const IeFixupTag * self,
                            const char * name,
                            IeFixupAttribute * attribute)
{
  size_t offset = self->attributes_offset;

  while (iefixup_tag_next_attribute (self, &offset, attribute))
  {
    if (strcmp (attribute->name, name) == 0)
      return TRUE;
  }

  return FALSE;
}

static BOOL
iefixup_tag_next_attribute (const IeFixupTag * self,
                            size_t * offset,
                            IeFixupAttribute * attribute)
{
  const unsigned char * data = self->data;
  size_t end = self->length - 1;
  size_t i = *offset;
  size_t start;
  unsigned char quote;

  while (i != end && (is_space (data[i]) || data[i] == '/'))
    i++;
  if (i == end)
    return FALSE;

  start = i;
  i = read_name (data, end, i, attribute->name);
  if (i == start)
    i++;
  attribute->value = data + i;
  attribute->value_length = 0;

  while (i != end && is_space (data[i]))
    i++;
  if (i != end && data[i] == '=')
  {
    i++;
    while (i != end && is_space (data[i]))
      i++;
    if (i != end && (data[i] == '"' || data[i] == '\''))
    {
      quote = data[i++];
      attribute->value = data + i;
      while (i != end && data[i] != quote)
        i++;
      attribute->value_length = data + i - attribute->value;
      if (i != end)
        i++;
    }
    else
    {
      attribute->value = data + i;
      while (i != end && !is_space (data[i]))
        i++;
      attribute->value_length = data + i - attribute->value;
    }
  }

  attribute->raw = data + start;
  attribute->raw_length = i - start;
  *offset = i;

  return TRUE;
}

static void
iefixup_tag_append_image_replacement (const IeFixupTag * self,
                                      IeFixupBuffer * output)
{
  IeFixupAttribute src, alt;

  if (iefixup_tag_find_attribute (self, "src", &src) && is_svg_url (src.value, src.value_length))
  {
    if (iefixup_tag_find_attribute (self, "alt", &alt))
      iefixup_utf8_append_as_cp1252 (output, alt.value, alt.value_length);
    return;
  }

  iefixup_buffer_append (output, "<img", 4);
  iefixup_tag_append_attributes (self, output);
  iefixup_buffer_append_byte (output, '>');
}

static void
iefixup_tag_append_attributes (const IeFixupTag * self,
                               IeFixupBuffer * output)
{
  size_t offset = self->attributes_offset;
  IeFixupAttribute attribute;

  while (iefixup_tag_next_attribute (self, &offset, &attribute))
  {
    if (!is_listed (attribute.name, allowed_attributes))
      continue;
    iefixup_buffer_append_byte (output, ' ');
    iefixup_utf8_append_as_cp1252 (output, attribute.raw, attribute.raw_length);
  }
}

static BOOL
is_svg_url (const unsigned char * url,
            size_t length)
{
  size_t end;

  for (end = 0; end != length && url[end] != '?' && url[end] != '#'; end++)
    ;

  return end >= 4 && url[end - 4] == '.' && (url[end - 3] | 0x20) == 's' && (url[end - 2] | 0x20) == 'v' &&
      (url[end - 1] | 0x20) == 'g';
}

static size_t
read_name (const unsigned char * data,
           size_t length,
           size_t offset,
           char * name)
{
  size_t n = 0;

  for (; offset != length && is_name_char (data[offset]); offset++)
  {
    if (n != MAX_NAME_LENGTH)
      name[n++] = data[offset] | ((data[offset] >= 'A' && data[offset] <= 'Z') ? 0x20 : 0);
  }
  name[n] = '\0';

  return offset;
}

static BOOL
is_name_char (unsigned char c)
{
  return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '-' || c == '_' ||
      c == ':';
}

static BOOL
is_space (unsigned char c)
{
  return c == ' ' || c == '\t' || c == '\r' || c == '\n' || c == '\f';
}

static BOOL
is_listed (const char * name,
           const char * const * list)
{
  for (; *list != NULL; list++)
  {
    if (strcmp (name, *list) == 0)
      return TRUE;
  }

  return FALSE;
}
