# Utility to import transaction from Vanguard into Newcash

SYSTEM!=uname
BIN=${HOME}/bin/${SYSTEM}

all: 
	cargo update && if test ${NEWCASH_DEBUG} -eq 1; then cargo build; else cargo build --release; fi

clean: 
	cargo clean

install:
	echo Installing to ${BIN}
	mkdir -p ${BIN}
	if test ${NEWCASH_DEBUG} -eq 1; then cp -f target/debug/vanguard_importer ${BIN}; else cp -f target/release/vanguard_importer ${BIN}; fi

uninstall:
	echo Removing from ${BIN}
	rm -f ${BIN}/vanguard_importer

.PHONY: all clean install uninstall 
