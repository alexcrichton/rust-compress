project('librust_compress', 'rust')

compresslib = shared_library('compress', 'src/compress.rs')

executable('testapp', 'src/testapp.rs', link_with : compresslib)

executable('gunzip', 'src/gunzip.rs', link_with : compresslib)
