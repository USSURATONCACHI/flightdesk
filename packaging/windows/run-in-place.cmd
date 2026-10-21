@echo off
cd /d "%~dp0"
set FLIGHTDECK_CONFIG_DIR=config
set FLIGHTDECK_DATA_DIR=data
set FLIGHTDECK_LOCALE_DIR=locales
start "" flightdeck.exe %*
