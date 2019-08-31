# Main application of the Newcash personal finance suite

SYSTEM!=uname
BIN=${HOME}/bin/${SYSTEM}

all:
	cargo update && if test ${NEWCASH_DEBUG} -eq 1; then cargo build; else cargo build --release; fi

clean: 
	cargo clean

install:
	echo Installing to ${BIN}
	mkdir -p ${BIN}
	if test ${NEWCASH_DEBUG} -eq 1; then cp -f target/debug/newcash ${BIN}; else cp -f target/release/newcash ${BIN}; fi
	rm -f ${HOME}/bin/newcash
	ln -s ${BIN}/newcash ${HOME}/bin/newcash

uninstall:
	echo Removing from ${BIN}
	rm -f ${BIN}/newcash
	rm -f ${HOME}/bin/newcash

.PHONY: all clean install uninstall 
