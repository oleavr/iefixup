#include "html.h"

#include <stdio.h>
#include <stdlib.h>

int
main (int argc,
      char * argv[])
{
  FILE * file;
  static unsigned char input[1 << 22];
  size_t size, offset, chunk;
  IeFixupHtml * html;
  IeFixupBuffer output;

  file = fopen (argv[1], "rb");
  size = fread (input, 1, sizeof (input), file);
  fclose (file);

  srand (atoi (argv[2]));
  html = iefixup_html_new ();
  iefixup_buffer_init (&output);

  for (offset = 0; offset != size; offset += chunk)
  {
    chunk = 1 + rand () % 700;
    if (chunk > size - offset)
      chunk = size - offset;
    iefixup_html_feed (html, input + offset, chunk, &output);
  }
  iefixup_html_finish (html, &output);

  fwrite (output.data, 1, output.length, stdout);

  iefixup_buffer_clear (&output);
  iefixup_html_free (html);

  return 0;
}
