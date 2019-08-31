# Utility to verify the structure of a Newcash database

SYSTEM!=uname
BIN=${HOME}/bin/${SYSTEM}

all: 
	cargo update && if test ${NEWCASH_DEBUG} -eq 1; then cargo build; else cargo build --release; fi

clean: 
	cargo clean

install:
	echo Installing to ${BIN}
	mkdir -p ${BIN}
	if test ${NEWCASH_DEBUG} -eq 1; then cp -f target/debug/verifier ${BIN}/newcashVerifier; else cp -f target/release/verifier ${BIN}/newcashVerifier; fi

uninstall:
	echo Removing from ${BIN}
	rm -f ${BIN}/verifier

.PHONY: all clean install uninstall 
