#ifndef __IEFIXUP_HTML_H__
#define __IEFIXUP_HTML_H__

#include "buffer.h"

typedef struct _IeFixupHtml IeFixupHtml;

IeFixupHtml * iefixup_html_new (void);
void iefixup_html_free (IeFixupHtml * self);

void iefixup_html_feed (IeFixupHtml * self, const unsigned char * data, size_t size, IeFixupBuffer * output);
void iefixup_html_finish (IeFixupHtml * self, IeFixupBuffer * output);

#endif
