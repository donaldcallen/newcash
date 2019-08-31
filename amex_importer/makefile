# Statement importer for the Newcash Personal Finance Manager 

EXE = newcashLoadTransactions.tcl newcashAMEXImporter

all: levenshtein.so

levenshtein.so: levenshtein.c
	gcc -g ${INCLUDES} -std=c99 -fPIC -shared levenshtein.c -o levenshtein.so

clean: 
	rm -f levenshtein.so

install:
	echo Installing scripts to ${HOME}/bin
	mkdir -p ${HOME}/bin
	cp -f ${EXE} ${HOME}/bin
	cp levenshtein.so ${HOME}/lib/${SYSTEM}

uninstall:
	echo Removing scripts from ${HOME}/bin
	cd ${HOME}/bin/ && rm -f ${EXE}

.PHONY: all clean install uninstall 
