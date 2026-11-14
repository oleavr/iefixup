#ifndef __IEFIXUP_LOG_H__
#define __IEFIXUP_LOG_H__

#if IEFIXUP_DEBUG
# define IEFIXUP_LOG(...) iefixup_log (__VA_ARGS__)
#else
# define IEFIXUP_LOG(...)
#endif

void iefixup_log (const char * format, ...);

#endif
