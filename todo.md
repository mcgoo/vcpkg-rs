# TODOs

add information about target triples
mention the fact that the default is static
make sure there is mention of the fact that the static or dynamic selection is done through the environment
make a note about the static/dynamic search algorithm, specifically that it is through the environment

RUSTFLAGS: -C target-feature=crt-static

make probe failure return a nonzero exit code so the build fails

remove crate doc info about the libname -> package mapping

allow manual triple selection
return nonzero from vcpkg_cli on probe failure

 COMMAND powershell -noprofile -executionpolicy Bypass -file ${_VCPKG_TOOLCHAIN_DIR}/msbuild/applocal.ps1
                        -targetBinary $<TARGET_FILE:${name}>
                        -installedDir "${_VCPKG_INSTALLED_DIR}/${VCPKG_TARGET_TRIPLET}$<$<CONFIG:Debug>:/debug>/bin"
                        -OutVariable out
)