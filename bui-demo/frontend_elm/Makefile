all: dist/js/main.js dist/js/index.js dist/index.html

elm-stuff/exact-dependencies.json: elm-package.json
	elm-package install -y

dist/js/main.js: src/Main.elm elm-stuff/exact-dependencies.json
	elm-make $< --output=$@

dist/js/index.js: src/index.js
	cp $< $@

dist/index.html: src/index.html
	cp $< $@

.PHONY: all
