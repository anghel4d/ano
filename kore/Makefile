# kore — これ, the ano editor. `make` builds ./kore. One cc invocation, whole-program,
# zero external deps — raw ANSI and termios, no ncurses; the strings module and its
# arena come from ../common, compiled in directly.
CC ?= cc
CFLAGS ?= -std=gnu23 -O2 -Wall -Wextra

COMMON = ../common
CSRC = $(COMMON)/ano_memory.c $(COMMON)/ano_strings.c $(COMMON)/ano_strings_ops.c \
       $(COMMON)/ano_strings_utf.c $(COMMON)/ano_strings_collate.c $(COMMON)/ano_strings_intern.c

all: kore

kore: kore.c $(CSRC) $(COMMON)/anoptic_strings.h $(COMMON)/anoptic_strings_utf.h $(COMMON)/anoptic_memory.h
	$(CC) $(CFLAGS) -I$(COMMON) -o $@ kore.c $(CSRC)

clean:
	rm -f kore

.PHONY: all clean
