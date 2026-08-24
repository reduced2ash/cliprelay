SHELL := /bin/bash

.PHONY: ui-test ui-test-config ui-test-clean ui-inspect ui-inspect-down

ui-test:
	@tools/ui-test/bin/run-host.sh

ui-test-config:
	@tools/ui-test/bin/config-host.sh

ui-test-clean:
	@tools/ui-test/bin/clean-host.sh

ui-inspect:
	@tools/ui-test/bin/inspect-host.sh

ui-inspect-down:
	@tools/ui-test/bin/inspect-host.sh down

