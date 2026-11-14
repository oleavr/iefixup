#include "forward.h"

void
iefixup_forward_init (HMODULE real)
{
  unsigned int i;

  for (i = 0; i != iefixup_n_forwards; i++)
    iefixup_forward_targets[i] = GetProcAddress (real, iefixup_forward_names[i]);
}
