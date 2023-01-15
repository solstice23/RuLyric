use druid::widget::{Button, Flex, Label};
use druid::{AppLauncher, LocalizedString, PlatformError, Widget, WidgetExt, WindowDesc, Data};

use crate::model::lyrics::LyricsData;
use crate::widgets::lyrics::LyricLine;

#[derive(Data,Clone,Debug)]
pub struct LyricAppData{
    pub current_lyric: String
}

pub fn ui_builder() -> impl Widget<LyricAppData> {
    let text = LyricLine::new(|data:&LyricAppData|{
        LyricsData::new(&data.current_lyric)
    });

    Flex::column().with_child(text)
}