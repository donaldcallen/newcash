# Utility to produce composite (transaction+splits) register

SYSTEM!=uname
BIN=${HOME}/bin/${SYSTEM}

all: 
	cargo update && if test ${NEWCASH_DEBUG} -eq 1; then cargo build; else cargo build --release; fi

clean: 
	cargo clean

install:
	echo Installing to ${BIN}
	mkdir -p ${BIN}
	if test ${NEWCASH_DEBUG} -eq 1; then cp -f target/debug/composite_register ${BIN}; else cp -f target/release/composite_register ${BIN}; fi

uninstall:
	echo Removing from ${BIN}
	rm -f ${BIN}/composite_register

.PHONY: all clean install uninstall 
