
build:
	@make -f script/riscv.mk build

run:
	@make -f script/riscv.mk run

fmt:
	@cargo fmt --all

clean:
	@cargo clean
	@make -f script/riscv.mk clean
	@cd ../user && make clean