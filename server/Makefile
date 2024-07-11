#!/usr/bin/make -f

BUILD_DIR = $(CURDIR)/build

###############################################################################
###                          Tools & Dependencies                           ###
###############################################################################

install-tools:
	@echo "--> Installing tools..."
	@go install \
		github.com/grpc-ecosystem/grpc-gateway/v2/protoc-gen-grpc-gateway \
		github.com/grpc-ecosystem/grpc-gateway/v2/protoc-gen-openapiv2 \
		google.golang.org/protobuf/cmd/protoc-gen-go \
		google.golang.org/grpc/cmd/protoc-gen-go-grpc

.PHONY: install-tools

###############################################################################
###                                Linting                                  ###
###############################################################################

golangci_version=v1.58.2

lint:
	@echo "--> Running linter..."
	@go install github.com/golangci/golangci-lint/cmd/golangci-lint@$(golangci_version)
	@golangci-lint run --config .golangci.yml --timeout 15m

lint-fix:
	@echo "--> Running linter and fixing errors..."
	@go install github.com/golangci/golangci-lint/cmd/golangci-lint@$(golangci_version)
	@golangci-lint run --config .golangci.yml --fix --timeout 15m

.PHONY: lint lint-fix

###############################################################################
###                                Protobuf                                 ###
###############################################################################

proto-all: proto-format proto-lint proto-gen

proto-gen:
	@sh scripts/protogen.sh

proto-format:
	@buf format ../proto/ -w

proto-lint:
	@buf lint ../proto/ --error-format=json

.PHONY: proto-all proto-gen proto-format proto-lint

###############################################################################
###                                  Mocks                                  ###
###############################################################################

mock-gen:
	@echo "--> Generating mocks..."
	@mockgen -source=pkg/types/zkvm_executor_grpc.pb.go -package mock -destination pkg/mock/zk_executor_client.go ZkvmExecutorClient

.PHONY: mock-gen

###############################################################################
###                                  Build                                  ###
###############################################################################

BUILD_TARGETS := build install

build: 
	go build -o $(BUILD_DIR)/

$(BUILD_DIR)/:
	mkdir -p $(BUILD_DIR)/

.PHONY: build build-linux-amd64 build-linux-arm64

clean:
	rm -rf \
	$(BUILD_DIR)/ 

.PHONY: clean