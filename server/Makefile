#!/usr/bin/make -f

export VERSION := $(shell echo $(shell git describe --always --match "v*") | sed 's/^v//')
export COMMIT := $(shell git log -1 --format='%H')
BUILDDIR ?= $(CURDIR)/build

###############################################################################
###                                  Build                                  ###
###############################################################################

ldflags = -X github.com/ethos-works/InfinityVM/server/pkg/cmd.Version=$(VERSION) \
		  -X github.com/ethos-works/InfinityVM/server/pkg/cmd.Commit=$(COMMIT)

BUILD_FLAGS := -ldflags '$(ldflags)'

BUILD_TARGETS := build

build: BUILD_ARGS=-o $(BUILDDIR)/infinity-server

$(BUILD_TARGETS): go.sum $(BUILDDIR)/
	@go $@ -mod=readonly $(BUILD_FLAGS) $(BUILD_ARGS) main.go

$(BUILDDIR)/:
	@mkdir -p $(BUILDDIR)/

.PHONY: build

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
	@mockgen -source=pkg/eth/eth_client.go -package=mock -destination=pkg/mock/eth_client.go EthClient

.PHONY: mock-gen
