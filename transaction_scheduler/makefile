# Utility to insert scheduled transactions

SYSTEM!=uname
BIN=${HOME}/bin/${SYSTEM}

all: 
	cargo update && if test ${NEWCASH_DEBUG} -eq 1; then cargo build; else cargo build --release; fi

clean: 
	cargo clean

install:
	echo Installing to ${BIN}
	mkdir -p ${BIN}
	if test ${NEWCASH_DEBUG} -eq 1; then cp -f target/debug/transaction_scheduler ${BIN}/newcashTransactionScheduler; else cp -f target/release/transaction_scheduler ${BIN}/newcashTransactionScheduler; fi

uninstall:
	echo Removing from ${BIN}
	rm -f ${BIN}/newcashTransactionScheduler

.PHONY: all clean install uninstall 
