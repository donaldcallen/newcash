# Newcash - Personal Finance Manager

SYSTEM != uname
NEWCASH_DEBUG = 1

all: newcash.m4 libSqliteExtensions.so
ifeq (${NEWCASH_DEBUG}, 1)
	cd newcash && cargo build
	cd composite_register && cargo build
	cd cambridge_trust_importer && cargo build
	cd report_generator/balance_sheet_income_expense_statement && cargo build
	cd report_generator/investments && cargo build
	cd verifier && cargo build
	cd vanguard_importer && cargo build
	cd transaction_scheduler && cargo build
else
	cd newcash && cargo build --release
	cd composite_register && cargo build --release
	cd cambridge_trust_importer && cargo build --release
	cd report_generator/balance_sheet_income_expense_statement && cargo build --release
	cd report_generator/investments && cargo build --release
	cd verifier && cargo build --release
	cd vanguard_importer && cargo build --release
	cd transaction_scheduler && cargo build --release
endif
	if test -d amex_importer; then cd amex_importer && make NEWCASH_DEBUG=${NEWCASH_DEBUG}; fi
	cd utilities && make all
	cd documentation && make all

newcash.m4: rust_library/src/constants.rs generateM4Include.awk
	awk -f generateM4Include.awk rust_library/src/constants.rs > newcash.m4

libSqliteExtensions.so: sqliteExtensions.c
	${CC} ${INCLUDES} -fPIC -lm -shared sqliteExtensions.c -o libSqliteExtensions.so

clean:
	rm -f newcash.m4 libSqliteExtensions.so
	cd newcash && cargo clean
	cd report_generator/balance_sheet_income_expense_statement && cargo clean
	cd report_generator/investments && cargo clean
	cd verifier && cargo clean
	cd composite_register && cargo clean
	cd cambridge_trust_importer && cargo clean
	cd amex_importer && make clean
	cd vanguard_importer && cargo clean
	cd transaction_scheduler && cargo clean
	cd utilities && make clean

install: newcash.m4 libSqliteExtensions.so
	mkdir -p ${HOME}/lib/${SYSTEM}
	cp libSqliteExtensions.so ${HOME}/lib/${SYSTEM}
ifeq (${NEWCASH_DEBUG}, 1)
	cd newcash && cargo install --debug --path . --force
	cd composite_register && cargo install --debug --path . --force
	cd cambridge_trust_importer && cargo install --debug --path . --force
	cd report_generator/balance_sheet_income_expense_statement && cargo install --debug --path . --force
	cd report_generator/investments && cargo install --debug --path . --force
	cd verifier && cargo install --debug --path . --force
	cd vanguard_importer && cargo install --debug --path . --force
	cd transaction_scheduler && cargo install --debug --path . --force
else
	cd newcash && cargo install --path . --force
	cd composite_register && cargo install --path . --force
	cd cambridge_trust_importer && cargo install --path . --force
	cd report_generator/balance_sheet_income_expense_statement && cargo install --path . --force
	cd report_generator/investments && cargo install --path . --force
	cd verifier && cargo install --path . --force
	cd vanguard_importer && cargo install --path . --force
	cd transaction_scheduler && cargo install --path . --force
endif
	cd amex_importer && make NEWCASH_DEBUG=${NEWCASH_DEBUG} install
	cd utilities && make install
	cd ~/bin ; rm -f newcashCambridgeTrustImporter; ln ../.cargo/bin/cambridge_trust_importer newcashCambridgeTrustImporter
	cd ~/bin ; rm -f newcashVerifier; ln ../.cargo/bin/verifier newcashVerifier
	cd ~/bin ; rm -f newcashTransactionScheduler; ln ../.cargo/bin/transaction_scheduler newcashTransactionScheduler

uninstall:
	rm -f ${HOME}/lib/${SYSTEM}/libSqliteExtensions.so
	cd newcash && cargo uninstall
	cd composite_register && cargo uninstall
	cd cambridge_trust_importer && cargo uninstall
	cd report_generator/balance_sheet_income_expense_statement && cargo uninstall
	cd report_generator/investments && cargo uninstall
	cd verifier && cargo uninstall
	cd vanguard_importer && cargo uninstall
	cd transaction_scheduler && cargo uninstall
	cd amex_importer && make uninstall
	cd utilities && make uninstall
	cd ~/bin ; rm newcashCambridgeTrustImporter
	cd ~/bin ; rm newcashVerifier
	cd ~/bin ; rm newcashTransactionScheduler

.PHONY: all clean install uninstall
