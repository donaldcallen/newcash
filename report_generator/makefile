# Newcash report generator

SYSTEM!=uname
BIN=${HOME}/bin/${SYSTEM}

all: 
	cargo update && if test ${NEWCASH_DEBUG} -eq 1; then cargo build; else cargo build --release; fi

clean: 
	cargo clean

install:
	echo Installing to ${BIN}
	mkdir -p ${BIN}
	if test ${NEWCASH_DEBUG} -eq 1; then cp -f target/debug/report_generator ${BIN}; else cp -f target/release/report_generator ${BIN}; fi

uninstall:
	echo Removing from ${BIN}
	rm -rf ${BIN}/report_generator

.PHONY: all clean install uninstall 
