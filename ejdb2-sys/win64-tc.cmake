if (WIN32)
    add_custom_target(wintools_init)

    macro(add_w32_importlib tgt libname wdir)
    add_custom_command(
        TARGET ${tgt}
        POST_BUILD
        COMMAND dlltool.exe -d ${libname}.def  -e ${libname}.exp -l ${libname}.lib -D ${libname}.dll
        WORKING_DIRECTORY ${wdir}
    )
    endmacro(add_w32_importlib)
endif()