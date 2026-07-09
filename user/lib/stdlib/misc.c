#include "../stdlib.h"
#include "../ctype.h"
#include "../limits.h"

#define uchar unsigned char
#define ulong unsigned long

int atoi(const char *s) {
  int n;

  n = 0;
  while ('0' <= *s && *s <= '9')
    n = n * 10 + *s++ - '0';
  return n;
}

long strtol(const char *nptr, char **endptr, int base) {
  const char *s;
  uchar uc;
  int c, neg, any;
  ulong acc, cutoff, cutlim;

  s = nptr;
  while (isspace((uchar)*s))
    s++;

  neg = 0;
  if (*s == '-' || *s == '+')
    neg = *s++ == '-';

  if ((base == 0 || base == 16) && *s == '0') {
    if (s[1] == 'x' || s[1] == 'X') {
      if (base == 0)
        base = 16;
      if (base == 16)
        s += 2;
    } else if (base == 0)
      base = 8;
  }
  if (base == 0)
    base = 10;
  if (base < 2 || base > 36) {
    if (endptr)
      *endptr = (char *)nptr;
    errno = EINVAL;
    return 0;
  }

  cutoff = neg ? (ulong)LONG_MIN : (ulong)LONG_MAX;
  cutlim = cutoff % (ulong)base;
  cutoff /= (ulong)base;

  for (acc = 0, any = 0;; s++) {
    uc = *s;
    if (isdigit(uc))
      c = uc - '0';
    else if (isupper(uc))
      c = uc - 'A' + 10;
    else if (islower(uc))
      c = uc - 'a' + 10;
    else
      break;
    if (c >= base)
      break;

    if (any < 0)
      continue;
    if (acc > cutoff || (acc == cutoff && (ulong)c > cutlim)) {
      any = -1;
      errno = ERANGE;
      continue;
    }
    any = 1;
    acc = acc * (ulong)base + (ulong)c;
  }

  if (!any) {
    if (endptr)
      *endptr = (char *)nptr;
    return 0;
  }
  if (endptr)
    *endptr = (char *)s;
  if (any < 0)
    return neg ? LONG_MIN : LONG_MAX;
  if (neg) {
    if (acc > (ulong)LONG_MAX + 1UL)
      return LONG_MIN;
    if (acc == (ulong)LONG_MAX + 1UL)
      return LONG_MIN;
    return -(long)acc;
  }
  return (long)acc;
}
