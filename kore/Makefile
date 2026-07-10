# kore — これ, the ano editor. `make` builds ./kore. One file, one cc invocation,
# whole-program, zero deps — raw ANSI and termios, no ncurses.
CC ?= cc
CFLAGS ?= -std=c23 -O2 -Wall -Wextra

all: kore

kore: kore.c
	$(CC) $(CFLAGS) -o $@ kore.c

clean:
	rm -f kore

.PHONY: all clean
