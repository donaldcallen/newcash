# Utility programs for the Newcash Personal Finance Manager 

BIN=${HOME}/bin

EXE = \
	newcashCommon \
	newcashCompositeRegister \
	newcashConvertGnucashDatabase \
	newcashCreateDatabase \
	newcashDuplicateTransactions \
	newcashGenerateReports \
	newcashIRR \
	newcashLoadQuotes \
	newcashReconciliationAid \
	newcashScheduledLoanPayment \
	newcashVanguardImporter \
	newcashVanguardPreProcessor.awk

M4 = \
	newcashConvertGnucashDatabase \
	newcashCreateDatabase \
	newcashIRR \
	newcashLoadQuotes  \
	newcashReconciliationAid \
	newcashScheduledLoanPayment \

all: ${M4}

%: %.m4 ../newcash.m4
	m4 -P $< > $@

clean:
	rm ${M4}

install:
	echo Installing executable file to ${BIN}
	chmod +x ${EXE}
	cp -f ${EXE} ${BIN}

uninstall:
	echo Removing scripts from ${BIN}
	cd ${BIN} && rm -rf ${EXE}

.PHONY: all clean install uninstall 
