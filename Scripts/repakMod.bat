@echo off
setlocal enabledelayedexpansion

:: if the mod folder exists before repacking, keep it for convenience (mostly for modders)
:: if folder was made during repacking phase, delete it after to avoid confusion/save space
set /a keepDir = 0

:: dont allow anything other than .pak files to be processed
set fileExt=none

:: check if anything was dropped into this script
if "%~1"=="" (
    echo No file or directory was provided. Don't forget: drag and drop the .pak file/folder into this script!
    pause
    exit /b
)

:: if more than one thing was provided, deal with them properly
for %%F in (%*) do (
    if exist "%%F" (
        if exist "%%F\*" (
            echo ======================================
            echo ======================================
            echo Processing folder: %%F
            @pushd %~dp0
            .\repak.exe --aes-key 0C263D8C22DCB085894899C3A3796383E9BF9DE0CBFB08C9BF2DEF2E84F29D74 pack "%%F" --version V11 --verbose --patch-uasset --compression Oodle
            @popd
        ) else (
			set "fileExt=%%~xF"
            :: convert to lowercase
            for %%A in (!fileExt!) do set "fileExt=%%A"
			
            if /i "!fileExt!"==".pak" (
				echo Processing .pak file: %%F
				
				if exist "%%~dpnF\*" ( set /a keepDir = 1 )
				
				@pushd %%~dpF
                .\repak.exe --aes-key 0C263D8C22DCB085894899C3A3796383E9BF9DE0CBFB08C9BF2DEF2E84F29D74 unpack "%%F" --verbose
				@del "%%F"
                .\repak.exe --aes-key 0C263D8C22DCB085894899C3A3796383E9BF9DE0CBFB08C9BF2DEF2E84F29D74 pack "%%~dpnF" --version V11 --verbose --patch-uasset --compression Oodle
                @popd
				
				if !keepDir! == 0 ( @RD /S /Q "%%~dpnF" )
            ) else ( echo File is nor a .pak file or a folder. )
		)
    )
)

pause
exit /b