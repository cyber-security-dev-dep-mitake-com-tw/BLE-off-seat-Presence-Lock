# (Tech Stack)

## 核心語言與執行環境
*   **語言**: Rust
    *   **優勢**: 提供強大的記憶體安全性、極低的 Runtime 開銷。Rust 的條件編譯 (`#[cfg(target_os = "windows")]`) 能完美切分 WinRT 與 CoreBluetooth 的底層實作，且能編譯為無依賴的單一執行檔 (Standalone Executable)，非常適合開發如 `watchdog.exe` 這種資訊安全相關的常駐代理程式。

## 藍牙底層通訊與作業系統 API
*   **跨平台 BLE 函式庫**: `btleplug` (Rust Crate)
    *   **角色**: 負責封裝底層藍牙掃描與裝置列舉。
    *   **Windows 底層對接**: 自動橋接 Windows.Devices.Bluetooth (WinRT API)，利用 BTHPORT 處理 IRK 解析。
    *   **macOS 底層對接**: 自動橋接 `CoreBluetooth` 框架，透過 `retrievePeripherals(withIdentifiers:)` 取得系統已信任的設備 `NSUUID`，並讀取 RSSI。
*   **Windows API 綁定**: `windows-rs` (Microsoft 官方 Rust 綁定)
    *   **用途**: 執行螢幕鎖定 (`LockWorkStation`) 與存取特定 WinRT 系統事件。
*   **macOS 系統呼叫**: `std::process::Command` 
    *   **用途**: 執行 `/System/Library/CoreServices/Menu Extras/User.menu/Contents/Resources/CGSession -suspend` 以觸發鎖定。

## 演算法與資料處理
*   **組態管理**: `serde` + `serde_yaml` (負責解析 `config.yaml`)
*   **濾波演算法**: 客製化 Exponential Moving Average (EMA) 
    *   **數學模型**: $S_t = \alpha \cdot Y_t + (1 - \alpha) \cdot S_{t-1}$
    *   $S_t$: 當前平滑後的 RSSI 值
    *   $Y_t$: 本次接收到的原始 RSSI 值
    *   $\alpha$: 平滑係數 (系統設定值)

## 參考技術文件 (References)
*   [Apple CoreBluetooth Documentation](https://developer.apple.com/documentation/corebluetooth) - 關於 `CBCentralManager` 與隱私授權的官方規範。
*   [Microsoft UWP Bluetooth LE Documentation](https://learn.microsoft.com/en-us/windows/uwp/devices-sensors/bluetooth-low-energy-overview) - 說明 `BluetoothLEAdvertisementWatcher` 與 IRK 處理機制的開發指南。
*   [btleplug GitHub Repository](https://github.com/deviceplug/btleplug) - Rust 跨平台藍牙核心實作細節與 OS 限制說明。