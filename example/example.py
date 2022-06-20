from ctypes import cdll
import os
import sys
libtinyevm = './target/release/libtinyevm.so'

if not os.path.exists(libtinyevm):
    print('Please build libtinyevm.so first with cargo build --release')
    sys.exit(1)

h = cdll.LoadLibrary(libtinyevm)

print(f'h.add(2, 3) => {h.add(2, 3)}')
