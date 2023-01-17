#[macro_use]
extern crate lazy_static;

static mut DATA_SENDER: Option<ExtEventSink> = None;
pub static mut WIN_HWND: Option<DWORD> = None;

use std::{fs, iter::once, os::windows::prelude::OsStrExt, sync::Arc};

use betterncm_macro::betterncm_native_call;
use betterncm_plugin_api::*;
use cef::CefV8Value;
use cef_sys::DWORD;
use druid::{
    AppDelegate, AppLauncher, Color, Command, DelegateCtx, Env, ExtEventSink, FontWeight, Handled,
    Selector, Target, WindowDesc, WindowHandle, WindowId,
};

use crate::{
    lyrics_app::{ui_builder, LyricAppData, LyricWinData},
    model::{
        font::FontConfig,
        lyrics::{LyricsData, LyricsWord},
    },
    win_helper::{embed_into_hwnd, get_desktop_hwnd},
};
mod lyrics_app;
mod model;
mod widgets;
mod win_helper;

struct Delegate {}

impl AppDelegate<LyricAppData> for Delegate {
    fn command(
        &mut self,
        ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        data: &mut LyricAppData,
        _env: &Env,
    ) -> Handled {
        if let Some(winid) = cmd.get(Selector::<usize>::new("CREATE_WINDOW")) {
            ctx.new_window(
                WindowDesc::new(ui_builder(*winid))
                    .show_titlebar(false)
                    .transparent(true)
                    .window_size((400.0, 70.0)),
            );
            Handled::Yes
        } else {
            Handled::No
        }
    }
}

fn edit_data(callback: impl FnOnce(&mut LyricAppData) + Send + std::marker::Sync + 'static) {
    unsafe {
        if let Some(sink) = DATA_SENDER.as_ref() {
            sink.add_idle_callback(|data: &mut LyricAppData| callback(data));
        }
    }
}

#[betterncm_native_call]
fn init_lyrics_app() {
    if unsafe { DATA_SENDER.is_none() } {
        std::thread::spawn(|| {
            let main_window = WindowDesc::new(ui_builder(0))
                .show_titlebar(true)
                .transparent(false)
                .window_size((400.0, 70.0));

            let app = AppLauncher::with_window(main_window)
                .delegate(Delegate {})
                .log_to_console();
            unsafe {
                DATA_SENDER = Some(app.get_external_handle());
            }

            app.launch(LyricAppData {
                current_lyric: LyricsData::new_test("".to_string()),
                current_lyric_ext: LyricsData::new_test("".to_string()),
                win_data: vec![LyricWinData {
                    font: FontConfig {
                        font_family: "Noto Sans SC".to_string(),
                        font_size: 18.,
                        font_color: druid::Color::WHITE,
                        font_weight: FontWeight::BOLD,
                    },
                    font_secondary: FontConfig {
                        font_family: "Noto Sans SC".to_string(),
                        font_size: 16.,
                        font_color: druid::Color::WHITE,
                        font_weight: FontWeight::NORMAL,
                    },
                    with_words_lyrics: false,
                }],
            })
        });
    } else {
        edit_data(|data| unsafe {
            data.win_data.push(LyricWinData {
                font: FontConfig {
                    font_family: "Noto Sans SC".to_string(),
                    font_size: 18.,
                    font_color: druid::Color::WHITE,
                    font_weight: FontWeight::BOLD,
                },
                font_secondary: FontConfig {
                    font_family: "Noto Sans SC".to_string(),
                    font_size: 16.,
                    font_color: druid::Color::WHITE,
                    font_weight: FontWeight::NORMAL,
                },
                with_words_lyrics: false,
            });
            let _ = DATA_SENDER.as_ref().unwrap().submit_command(
                Selector::new("CREATE_WINDOW"),
                data.win_data.len() - 1,
                Target::Global,
            );
        });
    }
}

#[betterncm_native_call]
fn update_lyrics(line: CefV8Value, line_ext: CefV8Value) {
    if line.is_string() {
        let line = line.get_string_value().to_string();
        edit_data(move |data: &mut LyricAppData| {
            data.current_lyric = LyricsData::new_test(line);
        });
    } else if line.is_object() {
        let line_num = line.get_value_byindex(1).get_uint_value();
        let words = line.get_value_byindex(0);
        let mut lyrics = vec![];
        for i in 0..words.get_array_length() {
            let val = words.get_value_byindex(i as isize);
            lyrics.push(LyricsWord {
                lyric_word: val.get_value_byindex(0).get_string_value().to_string(),
                lyric_duration: val.get_value_byindex(1).get_uint_value() as u64,
            });
        }

        if line_ext.is_string() {
            let line_ext = line_ext.get_string_value().to_string();
            edit_data(move |data: &mut LyricAppData| {
                data.current_lyric = LyricsData::from_lyrics(lyrics, line_num.try_into().unwrap());

                data.current_lyric_ext = LyricsData::from_text_duration(
                    line_ext,
                    data.current_lyric.get_full_duration(),
                );
            });
        } else {
            edit_data(move |data: &mut LyricAppData| {
                data.current_lyric = LyricsData::from_lyrics(lyrics, line_num.try_into().unwrap());
            });
        }
    }
}

#[betterncm_native_call]
fn embed_into_taskbar() {
    embed_into_with_classname(&"Shell_TrayWnd".to_string());
}

#[betterncm_native_call]
fn embed_into_desktop() {
    unsafe {
        embed_into_hwnd(get_desktop_hwnd());
    }
}

fn embed_into_with_classname(class_name: &String) {
    unsafe {
        use std::ptr::null_mut;
        let wide: Vec<u16> = std::ffi::OsStr::new(class_name)
            .encode_wide()
            .chain(once(0))
            .collect();
        let traywin = winapi::um::winuser::FindWindowW(wide.as_ptr(), null_mut());
        embed_into_hwnd(traywin as _);
    }
}

#[betterncm_native_call]
fn embed_into_any(class_name: CefV8Value) {
    embed_into_with_classname(&class_name.get_string_value().to_string());
}

#[betterncm_native_call]
fn seek(time: CefV8Value, paused: CefV8Value) {
    let time = time.get_uint_value() as u64;
    let paused = paused.get_bool_value();
    edit_data(move |data: &mut LyricAppData| {
        data.current_lyric.start_time = time;
        data.current_lyric.paused = paused;
    });
}

const FULL_V8VALUE_ARGS: [NativeAPIType; 100] = [NativeAPIType::V8Value; 100];

#[export_name = "BetterNCMPluginMain"]
extern "cdecl" fn betterncm_plugin_main(ctx: &mut PluginContext) -> ::core::ffi::c_int {
    unsafe {
        ctx.add_native_api_raw(
            FULL_V8VALUE_ARGS.as_ptr(),
            2,
            "rulyrics.update_lyrics\0".as_ptr() as _,
            update_lyrics,
        );

        ctx.add_native_api_raw(
            FULL_V8VALUE_ARGS.as_ptr(),
            0,
            "rulyrics.init_lyrics_app\0".as_ptr() as _,
            init_lyrics_app,
        );

        ctx.add_native_api_raw(
            FULL_V8VALUE_ARGS.as_ptr(),
            0,
            "rulyrics.embed_into_taskbar\0".as_ptr() as _,
            embed_into_taskbar,
        );

        ctx.add_native_api_raw(
            FULL_V8VALUE_ARGS.as_ptr(),
            0,
            "rulyrics.embed_into_desktop\0".as_ptr() as _,
            embed_into_desktop,
        );

        ctx.add_native_api_raw(
            FULL_V8VALUE_ARGS.as_ptr(),
            1,
            "rulyrics.embed_into_any\0".as_ptr() as _,
            embed_into_any,
        );

        ctx.add_native_api_raw(
            FULL_V8VALUE_ARGS.as_ptr(),
            2,
            "rulyrics.seek\0".as_ptr() as _,
            seek,
        );
    }

    println!("BetterNCM Rust Plugin loaded!");

    1
}
