# Minimal makefile for Sphinx documentation
#

# You can set these variables from the command line, and also
# from the environment for the first two.
SPHINXOPTS    ?=
SPHINXBUILD   ?= sphinx-build
SOURCEDIR     = source
BUILDDIR      = build
REDOC_OPT	  = 

# Put it first so that "make" without argument is like "make help".
help:
	@$(SPHINXBUILD) -M help "$(SOURCEDIR)" "$(BUILDDIR)" $(SPHINXOPTS) $(O)

.PHONY: help Makefile build

build:
	# redocly bundle --dereferenced --output dist ../web-api/openapi/front_office.yaml ../web-api/openapi/back_office.yaml 
	# redocly join dist/*.yaml -o combined.yaml
	# redocly build-docs combined.yaml -o combined.html $(REDOC_OPT)
	# redocly build-docs ../web-api/openapi/front_office.yaml -o front_office.html $(REDOC_OPT)
	# redocly build-docs ../web-api/openapi/back_office.yaml -o back_office.html $(REDOC_OPT)
	# redocly build-docs -o docs.html $(REDOC_OPT)
	# redocly build-docs ./common.yaml -o docs.html $(REDOC_OPT)
	@$(MAKE) singlehtml

# Catch-all target: route all unknown targets to Sphinx using the new
# "make mode" option.  $(O) is meant as a shortcut for $(SPHINXOPTS).
%: Makefile
	@$(SPHINXBUILD) -M $@ "$(SOURCEDIR)" "$(BUILDDIR)" $(SPHINXOPTS) $(O)
