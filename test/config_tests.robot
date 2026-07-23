*** Settings ***
Documentation     Config loading integration tests via the compiled binary.
Resource          resources/common.robot

*** Test Cases ***
Help Does Not Require Config
    [Documentation]    --help should succeed without a config file.
    Binary Should Exist
    ${result}=    Run Binary    --help
    Should Be Equal As Integers    ${result.rc}    0
    Should Contain    ${result.stdout}    headless
    Should Contain    ${result.stdout}    config
    Should Contain    ${result.stdout}    mock

Version Prints Package Version
    Binary Should Exist
    ${result}=    Run Binary    --version
    Should Be Equal As Integers    ${result.rc}    0
    Should Contain    ${result.stdout}    ble-presence-lock
    Should Contain    ${result.stdout}    0.1.0

Valid Config Starts In Mock Headless Mode
    [Documentation]    Valid YAML should allow the daemon to start.
    Binary Should Exist
    ${handle}=    Start Binary Background
    ...    --headless    --mock    --config    ${VALID_CONFIG}
    Sleep    2s
    ${running}=    Is Process Running    ${handle}
    Should Be True    ${running}
    Terminate Process    ${handle}    kill=True
    ${result}=    Wait For Process    ${handle}    timeout=5s
    # Process was killed; accept non-zero rc but require mock log output.
    ${output}=    Set Variable    ${result.stdout}
    Should Contain    ${output}    mock

Invalid Config Is Rejected
    Binary Should Exist
    ${result}=    Run Binary    --headless    --mock    --config    ${INVALID_CONFIG}
    Should Not Be Equal As Integers    ${result.rc}    0
    ${combined}=    Catenate    SEPARATOR=${\n}    ${result.stdout}    ${result.stderr}
    Should Contain Any    ${combined}    invalid config    ema_alpha    grace_period    log_level

Defaults Config Is Accepted
    [Documentation]    Minimal YAML with only device_name should apply defaults.
    Binary Should Exist
    ${handle}=    Start Binary Background
    ...    --headless    --mock    --config    ${DEFAULTS_CONFIG}
    Sleep    2s
    ${running}=    Is Process Running    ${handle}
    Should Be True    ${running}
    Terminate Process    ${handle}    kill=True
    ${result}=    Wait For Process    ${handle}    timeout=5s
    Should Contain    ${result.stdout}    TestPhone
