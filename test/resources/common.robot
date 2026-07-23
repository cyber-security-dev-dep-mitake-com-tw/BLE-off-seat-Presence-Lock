*** Settings ***
Documentation     Shared keywords and variables for BLE Presence Lock Robot tests.
Library           Process
Library           OperatingSystem
Library           String

*** Variables ***
${PROJECT_ROOT}       ${CURDIR}/../..
${BINARY}             ${PROJECT_ROOT}/target/debug/ble-presence-lock
${RESOURCES_DIR}      ${CURDIR}
${VALID_CONFIG}       ${RESOURCES_DIR}/valid_config.yaml
${INVALID_CONFIG}     ${RESOURCES_DIR}/invalid_config.yaml
${DEFAULTS_CONFIG}    ${RESOURCES_DIR}/defaults_config.yaml

*** Keywords ***
Binary Should Exist
    [Documentation]    Fail early if the debug binary has not been built.
    File Should Exist    ${BINARY}
    ...    msg=Binary not found at ${BINARY}. Run: cargo build

Run Binary
    [Arguments]    @{args}
    [Documentation]    Run the compiled binary with the given arguments.
    ${result}=    Run Process    ${BINARY}    @{args}
    ...    stdout=PIPE    stderr=PIPE    timeout=30s
    RETURN    ${result}

Start Binary Background
    [Arguments]    @{args}
    [Documentation]    Start the binary in the background and return the handle.
    ${handle}=    Start Process    ${BINARY}    @{args}
    ...    stdout=PIPE    stderr=STDOUT    alias=ble_app
    RETURN    ${handle}
