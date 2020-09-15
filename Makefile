.PHONY: all

all: api cli init loop jupyter

api:
	docker build -t onnovalkering/brane-api brane-api

cli:
	cargo build --release --package brane-cli

init:
	cargo build --release --package brane-init --target x86_64-unknown-linux-musl

jupyter:
	docker build -t onnovalkering/brane-jupyterlab brane-ide/jupyterlab

loop:
	docker build -t onnovalkering/brane-loop brane-loop