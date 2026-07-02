#!/bin/env bash

# export ANDROID_NDK_HOME=
if [ -z "$ANDROID_NDK_HOME" ]; then
    echo "Set ENV ANDROID_NDK_HOME"
    exit 1
fi

export CC_aarch64_linux_android=${ANDROID_NDK_HOME}/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android35-clang
export CXX_aarch64_linux_android=${ANDROID_NDK_HOME}/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android35-clang++
export AR_aarch64_linux_android=${ANDROID_NDK_HOME}/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar
export LD_aarch64_linux_android=${ANDROID_NDK_HOME}/toolchains/llvm/prebuilt/linux-x86_64/bin/ld
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=${CC_aarch64_linux_android}

export CC_x86_64_linux_android=${ANDROID_NDK_HOME}/toolchains/llvm/prebuilt/linux-x86_64/bin/x86_64-linux-android35-clang
export CXX_x86_64_linux_android=${ANDROID_NDK_HOME}/toolchains/llvm/prebuilt/linux-x86_64/bin/x86_64-linux-android35-clang++
export AR_x86_64_linux_android=${ANDROID_NDK_HOME}/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar
export LD_x86_64_linux_android=${ANDROID_NDK_HOME}/toolchains/llvm/prebuilt/linux-x86_64/bin/ld
export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER=${CC_x86_64_linux_android}

cargo build --target aarch64-linux-android --target x86_64-unknown-linux-gnu --target aarch64-unknown-linux-gnu --target riscv64gc-unknown-linux-gnu --target x86_64-unknown-linux-musl --target x86_64-pc-windows-gnu --release
