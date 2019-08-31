# Utility to import transaction from Cambridge Trust Wealth Management into Newcash

SYSTEM!=uname
BIN=${HOME}/bin/${SYSTEM}

all: 
	cargo update && if test ${NEWCASH_DEBUG} -eq 1; then cargo build; else cargo build --release; fi

clean: 
	cargo clean

install:
	echo Installing to ${BIN}
	mkdir -p ${BIN}
	if test ${NEWCASH_DEBUG} -eq 1; then cp -f target/debug/cambridge_trust_importer ${BIN}/newcashCambridgeTrustImporter; else cp -f target/release/cambridge_trust_importer ${BIN}/newcashCambridgeTrustImporter; fi

uninstall:
	echo Removing from ${BIN}
	rm -f ${BIN}/cambridge_trust_importer

.PHONY: all clean install uninstall 
