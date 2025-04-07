AC_INIT([fox32],
		[m4_esyscmd([build-aux/git-version-gen .tarball-version])],
		[https://github.com/fox32-arch])
AC_CONFIG_AUX_DIR([build-aux])
AM_INIT_AUTOMAKE([tar-ustar dist-zip foreign -Wall -Werror])
AC_PROG_CC
AC_PROG_SED

AC_CHECK_HEADER([fox32rom.h], [],
				[AC_MSG_ERROR([fox32rom.h is required but not found])])
PKG_CHECK_MODULES([SDL], [sdl2], [], [AC_MSG_ERROR([SDL2 is required but not found])])

AC_ARG_ENABLE([serial],
			  [AS_HELP_STRING([--disable-serial],
							  [Disable serial support @<:@enabled@:>@])],
			  [],
			  [enable_serial=yes])
AS_IF([test ! "x$enable_serial" = xno], [
	   AC_CHECK_HEADER([sys/select.h], [], [
						AC_MSG_ERROR([sys/select.h is required but not found])
						])
	   AC_CHECK_FUNC([tcgetattr], [], [
					  AC_MSG_ERROR([tcgetattr required but not found])
					  ])
	   ])

AM_CONDITIONAL([ENABLE_SERIAL], [test "x$enable_serial" != xno])
AC_CONFIG_HEADERS([config.h])
AC_CONFIG_FILES([Makefile src/Makefile])
AC_OUTPUT
