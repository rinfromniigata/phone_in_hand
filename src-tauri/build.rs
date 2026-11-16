fn main() {
    // OpenVRのC++ APIの薄い層（native/pih_openvr.cpp）。openvr_api.dll は実行時に読み込むため、
    // openvr_api.lib はリンクしない
    println!("cargo:rerun-if-changed=native/pih_openvr.cpp");
    println!("cargo:rerun-if-changed=vendor/openvr/headers/openvr.h");
    cc::Build::new()
        .cpp(true)
        .file("native/pih_openvr.cpp")
        .include("vendor/openvr/headers")
        .flag_if_supported("/std:c++17")
        .flag_if_supported("/EHsc")
        .flag_if_supported("/utf-8")
        .compile("pih_openvr");

    tauri_build::build()
}
