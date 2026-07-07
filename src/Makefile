# anoc — the ano -> BQN transpiler. `make` builds ./anoc, `make test` runs the demo corpus.
CC ?= cc
CFLAGS ?= -std=c23 -O2 -flto -Wall -Wextra
LDLIBS = -lm

OBJS = lex.o parse.o registry.o emit.o fs.o main.o

all: anoc

anoc: $(OBJS)
	$(CC) $(CFLAGS) -o $@ $(OBJS) $(LDLIBS)

$(OBJS): ano.h

test: anoc
	./check-ano.sh

clean:
	rm -f anoc $(OBJS)

.PHONY: all test clean
