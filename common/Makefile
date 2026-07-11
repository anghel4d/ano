# common/ — the anoptic strings module and its arena, shared by every C tree in this
# repo. `make` builds the objects, `make test` runs the smoke battery.
CC ?= cc
CFLAGS ?= -std=gnu23 -O2 -Wall -Wextra

OBJS = ano_memory.o ano_strings.o ano_strings_ops.o ano_strings_utf.o \
       ano_strings_collate.o ano_strings_intern.o

all: $(OBJS)

%.o: %.c anoptic_memory.h anoptic_strings.h anoptic_strings_utf.h ano_strings_internal.h
	$(CC) $(CFLAGS) -I. -c -o $@ $<

test: test_common
	./test_common

test_common: test_common.c $(OBJS)
	$(CC) $(CFLAGS) -I. -o $@ test_common.c $(OBJS)

clean:
	rm -f $(OBJS) test_common

.PHONY: all test clean
