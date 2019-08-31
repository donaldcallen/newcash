# Create constants for Rust from newcash.h

NEWCASH=..

all: src/lib.rs
	cargo update && if test ${NEWCASH_DEBUG} -eq 1; then cargo build; else cargo build --release; fi

clean: 

.PHONY: all clean
