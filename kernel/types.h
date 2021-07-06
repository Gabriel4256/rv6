#include <bits/types/struct_timeval.h>
#include <bits/types.h>

#ifndef TYPES_H_
#define TYPES_H_

typedef unsigned int   uint;
typedef unsigned short ushort;
typedef unsigned char  uchar;

typedef unsigned char uint8;
typedef unsigned short uint16;
typedef unsigned int  uint32;
typedef unsigned long uint64;

typedef uint64 pde_t;

typedef long int off_t;
typedef long unsigned int size_t;
typedef signed long int ssize_t;
typedef unsigned long u_long;

# define SEEK_SET	0	/* Seek from beginning of file.  */
# define SEEK_CUR	1	/* Seek from current position.  */
# define SEEK_END	2	/* Seek from end of file.  */

#ifndef stdin
#define stdin 0
#define stdout 1
#define stderr 2
#endif

#ifndef EOF
#define EOF -1
#endif

#ifndef NULL
#define NULL ((void *) 0)
#endif

#define __FD_SETSIZE 1024
typedef long int __fd_mask;
#define __NFDBITS	(8 * (int) sizeof (__fd_mask))

// TODO: IS this okay?
#define S_IREAD 0
#ifndef S_IWUSR
#define S_IWUSR 0
#endif

#ifndef S_IFIFO
#define S_IFIFO 0010000
#endif

#ifndef _SYS_SELECT_H
typedef struct fd_set{
  __fd_mask __fds_bits[__FD_SETSIZE / __NFDBITS];
} fd_set;
#endif

struct timezone {
	int	tz_minuteswest;	/* minutes west of Greenwich */
	int	tz_dsttime;	/* type of dst correction */
};

typedef __mode_t mode_t;



#define	SIGKILL	9	/* kill (cannot be caught or ignored) */
#define	SIGALRM	14	/* alarm clock */
#define	SIGTERM	15	/* software termination signal from kill */
#define	SIGCHLD	20	/* to parent on child stop or exit */
#define SIGUSR1 30	/* user defined signal 1 */

typedef void (*sighandler_t)(int);
#define	SIG_ERR	 ((sighandler_t) -1)	/* Error return.  */
#define	SIG_DFL	 ((sighandler_t)  0)	/* Default action.  */
#define	SIG_IGN	 ((sighandler_t)  1)	/* Ignore signal.  */

#ifndef __pid_t_defined
typedef uint pid_t;
#define __pid_t_defined
#endif

#endif