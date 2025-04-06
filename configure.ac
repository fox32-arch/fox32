# setup project
AC_INIT([fox32], [0.7.1], [https://github.com/fox32-arch])

# configure autotools
AC_CONFIG_AUX_DIR([build-aux])
AM_INIT_AUTOMAKE([foreign -Wall -Werror])
AC_CONFIG_HEADERS([config.h])
AC_CONFIG_FILES([Makefile src/Makefile])

# check for programs
AC_PROG_CC
AC_PROG_SED

# check for dependencies
AC_CHECK_HEADER([fox32rom.h], [],
  [AC_MSG_ERROR([fox32rom.h is required but not found])])
PKG_CHECK_MODULES([SDL], [sdl2], [], [AC_MSG_ERROR([SDL2 is required but not found])])

# write out files
AC_OUTPUT
