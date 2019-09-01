# Newcash - Personal Finance Manager 

SYSTEM != uname
NEWCASH_DEBUG = 0

all: newcash.m4 libSqliteExtensions.so
	cd newcash && make NEWCASH_DEBUG=${NEWCASH_DEBUG}
	cd rust_library && make NEWCASH_DEBUG=${NEWCASH_DEBUG}
	cd composite_register && make NEWCASH_DEBUG=${NEWCASH_DEBUG}
	if test -d cambridge_trust_importer; then cd cambridge_trust_importer && make NEWCASH_DEBUG=${NEWCASH_DEBUG}; fi
	cd report_generator && make  NEWCASH_DEBUG=${NEWCASH_DEBUG}
	cd verifier && make  NEWCASH_DEBUG=${NEWCASH_DEBUG}
	if test -d amex_importer; then cd amex_importer && make NEWCASH_DEBUG=${NEWCASH_DEBUG}; fi
	if test -d vanguard_importer; then cd vanguard_importer && make NEWCASH_DEBUG=${NEWCASH_DEBUG}; fi
	cd transaction_scheduler && make NEWCASH_DEBUG=${NEWCASH_DEBUG}
	cd utilities && make all
	cd documentation && make all

newcash.m4: rust_library/src/constants.rs generateM4Include.awk
	awk -f generateM4Include.awk rust_library/src/constants.rs > newcash.m4

libSqliteExtensions.so: sqliteExtensions.c
	${CC} ${INCLUDES} -fPIC -lm -shared sqliteExtensions.c -o libSqliteExtensions.so

clean: 
	rm -f newcash.m4 libSqliteExtensions.so
	cd newcash && make clean
	cd report_generator && make clean
	cd verifier && make clean
	cd composite_register && make clean
	if test -d cambridge_trust_importer; then cd cambridge_trust_importer && make clean; fi
	if test -d amex_importer; then cd amex_importer && make clean; fi
	if test -d vanguard_importer; then cd vanguard_importer && make clean; fi
	cd transaction_scheduler && make clean
	cd utilities && make clean

install:
	mkdir -p ${HOME}/lib/${SYSTEM}
	cp libSqliteExtensions.so ${HOME}/lib/${SYSTEM}
	cd newcash && make NEWCASH_DEBUG=${NEWCASH_DEBUG} install
	cd composite_register && make NEWCASH_DEBUG=${NEWCASH_DEBUG} install
	if test -d cambridge_trust_importer; then cd cambridge_trust_importer && make NEWCASH_DEBUG=${NEWCASH_DEBUG} install; fi
	cd verifier && make NEWCASH_DEBUG=${NEWCASH_DEBUG} install
	cd report_generator && make NEWCASH_DEBUG=${NEWCASH_DEBUG} install
	if test -d amex_importer; then cd amex_importer && make NEWCASH_DEBUG=${NEWCASH_DEBUG} install; fi
	if test -d vanguard_importer; then cd vanguard_importer && make NEWCASH_DEBUG=${NEWCASH_DEBUG} install; fi
	cd transaction_scheduler && make NEWCASH_DEBUG=${NEWCASH_DEBUG} install
	cd utilities && make install

uninstall:
	rm -f ${HOME}/lib/${SYSTEM}/libSqliteExtensions.so 
	cd composite_register && make uninstall
	if test -d cambridge_trust_importer; then cd cambridge_trust_importer && make uninstall; fi
	cd verifier && make uninstall
	cd report_generator && make uninstall
	cd newcash && make uninstall
	if test -d amex_importer; then cd amex_importer && make uninstall; fi
	if test -d vanguard_importer; then cd vanguard_importer && make uninstall; fi
	cd transaction_scheduler && make uninstall
	cd utilities && make uninstall

.PHONY: all clean install uninstall 
