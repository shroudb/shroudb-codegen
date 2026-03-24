.PHONY: test-clients clean

PROTOCOL_SPEC ?= $(realpath ../../shroudb/protocol.toml)

test-clients:
	@./run-tests.sh --spec "$(PROTOCOL_SPEC)"

clean:
	rm -rf generated/ .tmp/
