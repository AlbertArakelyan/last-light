# Force the static MSVC runtime (/MT) for CMake-built native deps like llama.cpp,
# so the shipped exe does not depend on the VC++ redistributable DLLs.
set(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreaded")
