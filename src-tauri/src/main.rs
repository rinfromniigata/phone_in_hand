// リリースのビルドで、余分なコンソールのウィンドウを出さない
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    phone_in_hand_lib::run();
}
