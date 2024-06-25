# reset-bst-linux-adapter

Under the cross-kernel driver framework, the Linux driver adaptation layer implemented for bsta1000b reset

## Clone repo

In your linux directory, clone this project

```shell
cd `path to your kernel`/drivers/reset
git clone git@github.com:OIZzzzzzz/reset-bst-linux-adapter.git
```

## Linux support Cargo 

The cross-kernel driver framework follows a componentized design and uses cargo to resolve component dependencies,
so it is necessary to add R4L support for cargo construction.
reference link: https://github.com/guoweikang/osl


## Add Makefile for adapter dir

Add this line into kernel/drivers/reset/Makefile

``` shell
obj-$(CONFIG_RESET_BSTA1000B_RUST) += reset-bst-linux-adapter/
```

Add this into kernel/drivers/reset/Kconfig

```shell
config RESET_BSTA1000B_RUST
	tristate "BST A1000B System Resource Reset in Rust"
	depends on ARCH_BSTA1000B && RUST
	default y
	help
	  This option enables support for the external reset functions for BST A1000B in rust.
```

**note**: if you want to use RUST driver,remeber disable C driver

