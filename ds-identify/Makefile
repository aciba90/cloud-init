toolchain_channel=+1.61

all: build

build:
	cargo $(toolchain_channel) build --verbose --release

install:
	install -D -m 755 target/release/ds-identify $(DESTDIR)/usr/lib/cloud-init/ds-identify

clean:
	cargo clean
