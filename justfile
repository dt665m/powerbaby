set dotenv-load
alias b := build 
alias s := server
alias c := client

# Path and Variables
ORG := "dt665m"
PROJECT := "powerbaby"
REPO := "https://github.com" / ORG / PROJECT
ROOT_DIR := justfile_directory()
OUTPUT_DIR := ROOT_DIR / "target"
SEM_VER := `awk -F' = ' '$1=="version"{print $2;exit;}' ./Cargo.toml`

default:
    @just --choose

semver:
	@echo {{SEM_VER}}

build:
    cargo build --release

client:
    target/release/powerbaby client

server:
    target/release/powerbaby server

single:
    target/release/powerbaby single
    

#NOTE zld did not install using `brew install zld` due to missing xcodebuild/xcode-select
