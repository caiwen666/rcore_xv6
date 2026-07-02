#include "../errno.h"
#include "../types.h"

static const struct {
  int errnum;
  const char *msg;
} errno_messages[] = {
    {ENOENT, "No such file or directory"},
    {EBADF, "Bad file descriptor"},
    {ENOMEM, "Not enough space"},
    {EFAULT, "Bad address"},
    {ENOTDIR, "Not a directory"},
    {EINVAL, "Invalid argument"},
    {ERANGE, "Result too large"},
    {ENAMETOOLONG, "Filename too long"},
};

char *strerror(int errnum) {
  for (usize i = 0; i < sizeof(errno_messages) / sizeof(errno_messages[0]); i++) {
    if (errno_messages[i].errnum == errnum) {
      return (char *)errno_messages[i].msg;
    }
  }
  return "Unknown error";
}
