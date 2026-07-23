*** Settings ***
Documentation     Binary CLI / mock / headless integration tests.
Resource          resources/common.robot

*** Test Cases ***
Prints Help
    Binary Should Exist
    ${result}=    Run Binary    --help
    Should Be Equal As Integers    ${result.rc}    0
    Should Contain    ${result.stdout}    BLE presence
    Should Contain    ${result.stdout}    --headless
    Should Contain    ${result.stdout}    --mock

Prints Version
    Binary Should Exist
    ${result}=    Run Binary    --version
    Should Be Equal As Integers    ${result.rc}    0
    Should Match Regexp    ${result.stdout}    ble-presence-lock\\s+0\\.1\\.0

Mock Mode Emits Scan Output
    [Documentation]    --mock --headless should log mock scan ticks.
    Binary Should Exist
    ${handle}=    Start Binary Background
    ...    --headless    --mock    --config    ${VALID_CONFIG}
    Sleep    3s
    Terminate Process    ${handle}    kill=True
    ${result}=    Wait For Process    ${handle}    timeout=5s
    Should Contain    ${result.stdout}    [mock]
    Should Contain    ${result.stdout}    rssi=

Headless Mode Starts And Shuts Down
    [Documentation]    Headless daemon should stay up then stop on terminate.
    Binary Should Exist
    ${handle}=    Start Binary Background
    ...    --headless    --mock    --config    ${VALID_CONFIG}
    Sleep    1s
    ${running}=    Is Process Running    ${handle}
    Should Be True    ${running}    msg=Headless process exited unexpectedly
    Terminate Process    ${handle}    kill=True
    ${result}=    Wait For Process    ${handle}    timeout=5s
    Should Contain    ${result.stdout}    headless monitor started

Missing Config File Fails Fast
    Binary Should Exist
    ${result}=    Run Binary
    ...    --headless    --mock    --config    /tmp/ble-presence-lock-missing-config.yaml
    Should Not Be Equal As Integers    ${result.rc}    0
    ${combined}=    Catenate    SEPARATOR=${\n}    ${result.stdout}    ${result.stderr}
    Should Contain    ${combined}    failed to load config
