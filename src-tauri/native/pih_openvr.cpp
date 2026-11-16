// OpenVRのC++ APIを、Rustから呼べるC関数にする薄い層（T3-1）。
//
// - openvr_api.dll は実行時に読み込む（LoadLibrary）。SteamVRや openvr_api.dll がない環境でもアプリは起動でき、
//   そのときは初期化が失敗を返すだけになる。openvr_api.lib はリンクしない
// - 呼び出しの直列化はRust側（src-tauri/src/openvr/）で行う。この層は状態として、読み込んだDLLと
//   インターフェースのポインタだけを持つ
// - 戻り値の int は、OpenVRの各エラーの列挙の値（0が成功）。この層の失敗は負の値で返す
#include <windows.h>

#include <cstdint>
#include <cstring>

#include "openvr.h"

using namespace vr;

namespace {

constexpr int kErrNotLoaded = -1;   // openvr_api.dll を読み込めない
constexpr int kErrNotInit = -2;     // 初期化していない
constexpr int kErrMissingFn = -3;   // DLLに関数がない
constexpr int kErrIface = -4;       // インターフェースを得られない

typedef uint32_t(VR_CALLTYPE* InitInternal2Fn)(EVRInitError*, EVRApplicationType, const char*);
typedef void(VR_CALLTYPE* ShutdownInternalFn)();
typedef void*(VR_CALLTYPE* GetGenericInterfaceFn)(const char*, EVRInitError*);
typedef bool(VR_CALLTYPE* IsInterfaceVersionValidFn)(const char*);
typedef bool(VR_CALLTYPE* IsRuntimeInstalledFn)();

HMODULE g_dll = nullptr;
InitInternal2Fn g_init = nullptr;
ShutdownInternalFn g_shutdown = nullptr;
GetGenericInterfaceFn g_get_iface = nullptr;
IsInterfaceVersionValidFn g_iface_valid = nullptr;
IsRuntimeInstalledFn g_runtime_installed = nullptr;

IVRSystem* g_system = nullptr;
IVROverlay* g_overlay = nullptr;
IVRApplications* g_apps = nullptr;
IVRInput* g_input = nullptr;

template <typename T>
T* iface(const char* version) {
    EVRInitError err = VRInitError_None;
    void* p = g_get_iface(version, &err);
    return err == VRInitError_None ? static_cast<T*>(p) : nullptr;
}

void to_matrix(const float* m, HmdMatrix34_t* out) {
    for (int r = 0; r < 3; r++)
        for (int c = 0; c < 4; c++) out->m[r][c] = m[r * 4 + c];
}

void from_matrix(const HmdMatrix34_t& in, float* m) {
    for (int r = 0; r < 3; r++)
        for (int c = 0; c < 4; c++) m[r * 4 + c] = in.m[r][c];
}

}  // namespace

extern "C" {

// ---- 読み込みと接続 ----

// openvr_api.dll を読み込む。path はDLLの絶対パス（UTF-16、NUL終端）
int pih_vr_load(const wchar_t* path) {
    if (g_dll) return 0;
    HMODULE dll = LoadLibraryW(path);
    if (!dll) return kErrNotLoaded;
    g_init = reinterpret_cast<InitInternal2Fn>(GetProcAddress(dll, "VR_InitInternal2"));
    g_shutdown = reinterpret_cast<ShutdownInternalFn>(GetProcAddress(dll, "VR_ShutdownInternal"));
    g_get_iface = reinterpret_cast<GetGenericInterfaceFn>(GetProcAddress(dll, "VR_GetGenericInterface"));
    g_iface_valid = reinterpret_cast<IsInterfaceVersionValidFn>(GetProcAddress(dll, "VR_IsInterfaceVersionValid"));
    g_runtime_installed = reinterpret_cast<IsRuntimeInstalledFn>(GetProcAddress(dll, "VR_IsRuntimeInstalled"));
    if (!g_init || !g_shutdown || !g_get_iface || !g_iface_valid || !g_runtime_installed) {
        FreeLibrary(dll);
        return kErrMissingFn;
    }
    g_dll = dll;
    return 0;
}

// SteamVRが入っているか（1/0）。読み込む前は0
int pih_vr_runtime_installed() {
    return g_dll && g_runtime_installed() ? 1 : 0;
}

// オーバーレイのアプリとして接続する。戻り値は EVRInitError
int pih_vr_init() {
    if (!g_dll) return kErrNotLoaded;
    if (g_system) return 0;
    EVRInitError err = VRInitError_None;
    g_init(&err, VRApplication_Overlay, nullptr);
    if (err != VRInitError_None) return err;
    if (!g_iface_valid(IVRSystem_Version) || !g_iface_valid(IVROverlay_Version)) {
        g_shutdown();
        return VRInitError_Init_InterfaceNotFound;
    }
    IVRSystem* sys = iface<IVRSystem>(IVRSystem_Version);
    IVROverlay* ov = iface<IVROverlay>(IVROverlay_Version);
    if (!sys || !ov) {
        g_shutdown();
        return kErrIface;
    }
    // VR_Init と同じく、使っているSDKの版を伝える
    EVRInitError ver = sys->SetSDKVersion(k_nSteamVRVersionMajor, k_nSteamVRVersionMinor, k_nSteamVRVersionBuild);
    if (ver != VRInitError_None) {
        g_shutdown();
        return ver;
    }
    g_system = sys;
    g_overlay = ov;
    g_apps = g_iface_valid(IVRApplications_Version) ? iface<IVRApplications>(IVRApplications_Version) : nullptr;
    g_input = g_iface_valid(IVRInput_Version) ? iface<IVRInput>(IVRInput_Version) : nullptr;
    return 0;
}

void pih_vr_shutdown() {
    if (!g_system) return;
    g_system = nullptr;
    g_overlay = nullptr;
    g_apps = nullptr;
    g_input = nullptr;
    g_shutdown();
}

// イベントを読み切る。SteamVRが終了を求めたら1を返す（このとき終了の受け入れも伝える）
int pih_vr_poll_quit() {
    if (!g_system) return 0;
    VREvent_t ev;
    int quit = 0;
    while (g_system->PollNextEvent(&ev, sizeof(ev))) {
        if (ev.eventType == VREvent_Quit) quit = 1;
    }
    if (quit) g_system->AcknowledgeQuit_Exiting();
    return quit;
}

// 次のSteamVRのフレームまで待つ。戻り値は EVROverlayError
int pih_vr_wait_frame_sync(uint32_t timeout_ms) {
    if (!g_overlay) return kErrNotInit;
    return g_overlay->WaitFrameSync(timeout_ms);
}

// ---- 姿勢 ----

// 左右のコントローラーの、次に表示されるときの予測の姿勢（standing空間）を得る。
// side: 0=左、1=右。m は 3x4 の行列（行優先）。戻り値: 1=追跡中、0=追跡が切れている・つながっていない
int pih_vr_controller_pose(int side, float* m) {
    if (!g_system) return 0;
    TrackedDeviceIndex_t idx = g_system->GetTrackedDeviceIndexForControllerRole(
        side == 0 ? TrackedControllerRole_LeftHand : TrackedControllerRole_RightHand);
    if (idx == k_unTrackedDeviceIndexInvalid || idx >= k_unMaxTrackedDeviceCount) return 0;

    // 表示までの時間を予測する（OpenVRの GetDeviceToAbsoluteTrackingPose の説明にある求め方）
    float since_vsync = 0.0f;
    g_system->GetTimeSinceLastVsync(&since_vsync, nullptr);
    float freq = g_system->GetFloatTrackedDeviceProperty(k_unTrackedDeviceIndex_Hmd, Prop_DisplayFrequency_Float);
    float vsync_to_photons =
        g_system->GetFloatTrackedDeviceProperty(k_unTrackedDeviceIndex_Hmd, Prop_SecondsFromVsyncToPhotons_Float);
    float predicted = (freq > 0.0f ? 1.0f / freq : 0.0f) - since_vsync + vsync_to_photons;
    if (predicted < 0.0f) predicted = 0.0f;

    TrackedDevicePose_t poses[k_unMaxTrackedDeviceCount];
    g_system->GetDeviceToAbsoluteTrackingPose(TrackingUniverseStanding, predicted, poses, k_unMaxTrackedDeviceCount);
    const TrackedDevicePose_t& p = poses[idx];
    if (!p.bDeviceIsConnected || !p.bPoseIsValid || p.eTrackingResult != TrackingResult_Running_OK) return 0;
    from_matrix(p.mDeviceToAbsoluteTracking, m);
    return 1;
}

// ---- オーバーレイ ----

int pih_overlay_create(const char* key, const char* name, uint64_t* handle) {
    if (!g_overlay) return kErrNotInit;
    VROverlayHandle_t h = k_ulOverlayHandleInvalid;
    EVROverlayError err = g_overlay->CreateOverlay(key, name, &h);
    if (err == VROverlayError_KeyInUse) {
        // 前回の接続で作ったものが残っていれば、それを使う
        err = g_overlay->FindOverlay(key, &h);
    }
    *handle = h;
    return err;
}

int pih_overlay_set_visible(uint64_t h, int visible) {
    if (!g_overlay) return kErrNotInit;
    return visible ? g_overlay->ShowOverlay(h) : g_overlay->HideOverlay(h);
}

int pih_overlay_set_width(uint64_t h, float meters) {
    if (!g_overlay) return kErrNotInit;
    return g_overlay->SetOverlayWidthInMeters(h, meters);
}

int pih_overlay_set_alpha(uint64_t h, float alpha) {
    if (!g_overlay) return kErrNotInit;
    return g_overlay->SetOverlayAlpha(h, alpha);
}

int pih_overlay_set_color(uint64_t h, float r, float g, float b) {
    if (!g_overlay) return kErrNotInit;
    return g_overlay->SetOverlayColor(h, r, g, b);
}

int pih_overlay_set_sort_order(uint64_t h, uint32_t order) {
    if (!g_overlay) return kErrNotInit;
    return g_overlay->SetOverlaySortOrder(h, order);
}

int pih_overlay_set_ignore_alpha(uint64_t h, int on) {
    if (!g_overlay) return kErrNotInit;
    return g_overlay->SetOverlayFlag(h, VROverlayFlags_IgnoreTextureAlpha, on != 0);
}

// m は 3x4 の行列（行優先、standing空間）
int pih_overlay_set_transform(uint64_t h, const float* m) {
    if (!g_overlay) return kErrNotInit;
    HmdMatrix34_t mat;
    to_matrix(m, &mat);
    return g_overlay->SetOverlayTransformAbsolute(h, TrackingUniverseStanding, &mat);
}

// texture は ID3D11Texture2D*
int pih_overlay_set_texture_d3d11(uint64_t h, void* texture) {
    if (!g_overlay) return kErrNotInit;
    Texture_t t;
    t.handle = texture;
    t.eType = TextureType_DirectX;
    t.eColorSpace = ColorSpace_Auto;
    return g_overlay->SetOverlayTexture(h, &t);
}

// テクスチャのうち表示する範囲（0〜1。切り抜き）
int pih_overlay_set_bounds(uint64_t h, float u_min, float v_min, float u_max, float v_max) {
    if (!g_overlay) return kErrNotInit;
    VRTextureBounds_t b;
    b.uMin = u_min;
    b.vMin = v_min;
    b.uMax = u_max;
    b.vMax = v_max;
    return g_overlay->SetOverlayTextureBounds(h, &b);
}

// RGBA（1画素4バイト）の画像を設定する。小さな画像（状態のチップ）だけに使う
int pih_overlay_set_raw(uint64_t h, void* rgba, uint32_t width, uint32_t height) {
    if (!g_overlay) return kErrNotInit;
    return g_overlay->SetOverlayRaw(h, rgba, width, height, 4);
}

// ---- 入力（位置合わせ） ----

// アクションの定義のファイル（絶対パス、UTF-8）を読み、ハンドルを得る。
// handles: [アクションセット, つかむ, 決定, 左手, 右手]
int pih_input_init(const char* manifest_path, uint64_t* handles) {
    if (!g_input) return kErrNotInit;
    EVRInputError err = g_input->SetActionManifestPath(manifest_path);
    if (err != VRInputError_None) return err;
    if ((err = g_input->GetActionSetHandle("/actions/calibrate", &handles[0])) != VRInputError_None) return err;
    if ((err = g_input->GetActionHandle("/actions/calibrate/in/grab", &handles[1])) != VRInputError_None) return err;
    if ((err = g_input->GetActionHandle("/actions/calibrate/in/confirm", &handles[2])) != VRInputError_None) return err;
    if ((err = g_input->GetInputSourceHandle("/user/hand/left", &handles[3])) != VRInputError_None) return err;
    if ((err = g_input->GetInputSourceHandle("/user/hand/right", &handles[4])) != VRInputError_None) return err;
    return 0;
}

// アクションの状態を更新し、指定した手の「つかむ」「決定」が押されているかを返す
int pih_input_poll(const uint64_t* handles, int side, int* grab, int* confirm) {
    if (!g_input) return kErrNotInit;
    VRActiveActionSet_t set;
    std::memset(&set, 0, sizeof(set));
    set.ulActionSet = handles[0];
    set.ulRestrictedToDevice = k_ulInvalidInputValueHandle;
    EVRInputError err = g_input->UpdateActionState(&set, sizeof(set), 1);
    if (err != VRInputError_None) return err;
    VRInputValueHandle_t device = side == 0 ? handles[3] : handles[4];
    InputDigitalActionData_t d;
    *grab = 0;
    *confirm = 0;
    if (g_input->GetDigitalActionData(handles[1], &d, sizeof(d), device) == VRInputError_None)
        *grab = d.bActive && d.bState ? 1 : 0;
    if (g_input->GetDigitalActionData(handles[2], &d, sizeof(d), device) == VRInputError_None)
        *confirm = d.bActive && d.bState ? 1 : 0;
    return 0;
}

// ---- アプリの登録（SteamVRと一緒に起動する） ----

int pih_apps_is_installed(const char* key) {
    return g_apps && g_apps->IsApplicationInstalled(key) ? 1 : 0;
}

int pih_apps_add_manifest(const char* path) {
    if (!g_apps) return kErrNotInit;
    return g_apps->AddApplicationManifest(path, false);
}

int pih_apps_remove_manifest(const char* path) {
    if (!g_apps) return kErrNotInit;
    return g_apps->RemoveApplicationManifest(path);
}

int pih_apps_set_auto_launch(const char* key, int on) {
    if (!g_apps) return kErrNotInit;
    return g_apps->SetApplicationAutoLaunch(key, on != 0);
}

int pih_apps_get_auto_launch(const char* key) {
    return g_apps && g_apps->GetApplicationAutoLaunch(key) ? 1 : 0;
}

}  // extern "C"
