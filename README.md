# BLE Presence Lock

BLE Presence Lock 是一支跨平台 (Windows / macOS) 的系統常駐背景程式 (Daemon)。透過持續監聽已配對藍牙裝置 (手機、智慧手錶) 的接收訊號強度指標 (RSSI)，判斷使用者是否位於電腦前。當裝置遠離導致 RSSI 衰減低於安全閾值時，系統將自動鎖定螢幕以確保資訊安全。

## 核心機制 (How it Works)
本系統**不需要**在手機或手錶端安裝任何專屬 App。
系統完全依賴作業系統層級的藍牙配對機制。裝置與電腦配對後交換的 IRK (Identity Resolving Key) 會交由 Windows BTHPORT 與 macOS CoreBluetooth 底層處理。即使現代行動裝置預設開啟 MAC 位址隨機化 (MAC Address Randomization)，系統仍能穩定識別並追蹤特定受信任的設備。

## 主要功能 (Features)
*   **無代理程式 (Agentless on Mobile)**：支援 iOS, Android, Apple Watch, Garmin 等任何支援標準 BLE 配對的裝置。
*   **跨平台支援**：原生支援 Windows 10/11 與 macOS。
*   **抗干擾濾波**：內建指數移動平均 (EMA) 演算法與時間延遲鎖定 (Grace Period) 機制，大幅降低因訊號多徑效應或短暫遮蔽造成的誤鎖率。
*   **低系統消耗**：純背景服務運行，無龐大 GUI，極低 CPU 佔用。

## 安裝與設定 (Setup & Configuration)
1.  **配對裝置**：在 Windows 或 macOS 系統的藍牙設定中，以標準方式將您的手機或手錶與電腦完成配對。
2.  **設定閾值 (`config.yaml`)**：
    ```yaml
    target_device_name: "My iPhone" # 配對時顯示的裝置名稱
    rssi_threshold: -80             # 觸發鎖定的 RSSI 閾值 (dBm)
    grace_period_sec: 10            # 訊號低於閾值後的緩衝時間 (秒)
    ema_alpha: 0.2                  # EMA 濾波係數 (0.1 ~ 1.0，越小越平滑)
    ```
3.  **啟動服務**：執行編譯後的可執行檔。

# 專案目錄結構 (Project Structure)

```text
ble-presence-lock/
├── .github(gha pipelines ,編譯成mac os/windows os的dmg/pkg/exe/msi )
├── src/
│   ├── main.rs                 # 程式進入點，負責載入設定檔與啟動監聽迴圈
│   ├── config.rs               # yaml 設定檔解析與常數定義
│   │
│   ├── ble/                    # 藍牙通訊抽象層
│   │   ├── mod.rs              # 定義跨平台共用的 Scanner Trait (介面)
│   │   ├── windows.rs          # WinRT API 實作 (BluetoothLEAdvertisementWatcher)
│   │   └── macos.rs            # CoreBluetooth API 實作 (CBCentralManager)
│   │
│   ├── filter/                 # 訊號處理與演算法
│   │   ├── mod.rs
│   │   ├── ema.rs              # 指數移動平均濾波 (Exponential Moving Average) 實作
│   │   └── debouncer.rs        # 狀態機：處理 Grace period 倒數與觸發邏輯
│   │
│   └── os_action/              # 作業系統操作
│   │   ├── mod.rs
│   │   ├── locker_win.rs       # 呼叫 user32.dll 的 LockWorkStation()
│   │   └── locker_mac.rs       # 呼叫 macOS CGSession -suspend 鎖定螢幕
│   └── UI/                       # 非常簡易的app lite ui (使用julia)
│
├── Cargo.toml                  # 套件與依賴管理設定
├── config.example.yaml         # 預設設定檔範本
└── docs/                       # 技術說明文件與架構圖
```