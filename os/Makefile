
build:
	@make -f script/riscv.mk build

fs-img:
	@make -f script/riscv.mk fs-img

user:
	@cd ../user && make build

run:
	@make -f script/riscv.mk run

fmt:
	@cargo fmt --all

clean:
	@cargo clean
	@cd ../user && make clean
	@rm -rf ../fs-img