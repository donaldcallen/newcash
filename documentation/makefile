all: newcashUserManual.pdf

newcashUserManual.pdf: newcashUserManual.tex
	pdflatex newcashUserManual.tex

install: newcashUserManual.pdf
	echo installing manual to ${PREFIX}/share/doc
	mkdir -p ${PREFIX}/share/doc
	mkdir -p ${PREFIX}/share/doc/newcash
	cp -f newcashUserManual.pdf ${PREFIX}/share/doc/newcash

uninstall:
	echo removing  manual from ${PREFIX}/share/doc
	rm -rf ${PREFIX}/share/doc/newcash
