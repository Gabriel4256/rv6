// #include <stdio.h>
// #include <stdlib.h>
#include "kernel/types.h"
#include "kernel/stat.h"
#include "user/user.h"

# define __FDS_BITS(set) ((set)->__fds_bits)
#define FD_ZERO(set) \
  do {									      \
    unsigned int __i;							      \
    fd_set *__arr = (set);						      \
    for (__i = 0; __i < sizeof (fd_set) / sizeof (__fd_mask); ++__i)	      \
      __FDS_BITS (__arr)[__i] = 0;					      \
  } while (0)

#define __NFDBITS	(8 * (int) sizeof (__fd_mask))
#define	__FD_ELT(d)	((d) / __NFDBITS)
#define	__FD_MASK(d)	((__fd_mask) (1UL << ((d) % __NFDBITS)))

#define FD_SET(d, set) \
  ((void) (__FDS_BITS (set)[__FD_ELT (d)] |= __FD_MASK (d)))


#include <stdlib.h>
#include <stdarg.h>
#include "nscanf.h"

#define EOS_MATCHER_CHAR	'\f'




int main(){
    fprintf(2, "This is a test program\n");

    fd_set rfds;
    int timeval = 10;
    int ret;
    FD_ZERO(&rfds);




    int n;
    int filedes[2];
    char buffer[1025];
    char *message = "Hello, World!";

    pipe(filedes);
    write(filedes[1], message, strlen(message));

    FD_SET(filedes[0], &rfds);
    fprintf(1, "select pipe read test with fd: %d\n", filedes[0]);
    ret = select(filedes[0] + 1, &rfds, 0, 0, timeval);
    fprintf(1, "result: %d\n", ret);

    fprintf(1, "select timeout test\n");
    ret = select(1, 0, 0, 0, timeval);
    fprintf(1, "result: %d\n", ret);

    if ((n = read ( filedes[0], buffer, 1024 ) ) >= 0) {
        buffer[n] = 0;  //terminate the string
        fprintf(1, "read %d bytes from the pipe: %s\n", n, buffer);
    }  
    else
        fprintf(2, "read\n");


    int pgsize = getpagesize();
    printf("page size: %d\n", pgsize);

  /// sprintf test

    int integer = 123;
  char character = 'g';
  char string[30] = "hello, world";
  int* pointer = &integer;
  // double pi = 3.141592;
  char buf[100];

  printf("sprintf test\n");
  sprintf(buf, "integer : (decimal) %d (octal) %o \n", integer, integer);
  printf("%s \n", buf);

  sprintf(buf, "character : %c \n", character);
  printf("%s \n", buf);

  sprintf(buf, "string : %s \n", string);
  printf("%s \n", buf);

  sprintf(buf, "pointer addr : %p \n", pointer);
  printf("%s \n", buf);

  // sprintf(buf, "floating point : %e // %f \n", pi, pi);
  // printf("%s \n", buf);

  sprintf(buf, "percent symbol : %% \n");
  printf("%s \n", buf);


  char str[30] = "1234";
  int i;

  sscanf(str, "%d", &i);

  printf("Number from : '%s' \n", str);
  printf("number : %d \n", i);


  exit(0);
    
}