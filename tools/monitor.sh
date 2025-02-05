#!/bin/bash

: "${MONITOR_PORT:="55555"}"

function powerdown() {
    echo powering down VM...
    echo system_powerdown | netcat -N localhost "$MONITOR_PORT"
}

function savevm() {
    echo saving vm snapshot with tag "$2"
    echo "savevm $2" | netcat -N localhost "$MONITOR_PORT"
}

function loadvm() {
    echo loading vm snapshot with tag "$2"
    echo "loadvm $2" | netcat -N localhost "$MONITOR_PORT"
}

FUNC_CALL=$1; shift; $FUNC_CALL "$@"
