use std::{collections::HashMap, sync::LazyLock};

use once_cell::sync::OnceCell;
use ratatui::{
    buffer::Buffer,
    layout::{Margin, Rect},
    widgets::Widget,
};
use sysinfo::System;

use crate::ui::build::normal_block;

#[derive(Debug, Clone)]
struct OsIcon {
    name: &'static str,
    logo: [&'static str; 20],
    color: Vec<Color>,
}

#[derive(Debug, Clone, Copy)]
struct Color {
    char: char,
    color: ratatui::style::Color,
}

#[derive(Debug, Clone, Copy)]
struct CachedGlyph {
    char: char,
    color: ratatui::style::Color,
}

#[derive(Debug, Clone, Default)]
struct RenderedIcon {
    lines: Vec<Vec<CachedGlyph>>,
}

static OS_ICONS: LazyLock<[OsIcon; 6]> = std::sync::LazyLock::new(|| {
    [
        OsIcon {
            name: "ubuntu",
            logo: [
                "                             ....        ",
                "              $2.',:clooo:  $1.:looooo:. ",
                "           $2.;looooooooc  $1.oooooooooo'",
                "        $2.;looooool:,''.  $1:ooooooooooc",
                "       $2;looool;.         $1'oooooooooo,",
                "      $2;clool'             $1.cooooooc.  $2,, ",
                "         $2...                $1......  $2.:oo,",
                "  $1.;clol:,.                        $2.loooo' ",
                " $1:ooooooooo,                        $2'ooool ",
                "$1'ooooooooooo.                        $2loooo.",
                "$1'ooooooooool                         $2coooo.",
                " $1,loooooooc.                        $2.loooo.",
                "   $1.,;;;'.                          $2;ooooc ",
                "       $2...                         $2,ooool. ",
                "    $2.cooooc.              $1..',,'.  $2.cooo.",
                "      $2;ooooo:.           $1;oooooooc.  $2:l. ",
                "       $2.coooooc,..      $1coooooooooo.",
                "         $2.:ooooooolc:. $1.ooooooooooo'",
                "           $2.':loooooo;  $1,oooooooooc ",
                "               $2..';::c'  $1.;loooo:'  ",
            ],
            color: Vec::from(&[
                Color {
                    char: 'o',
                    color: ratatui::style::Color::Red,
                },
                Color {
                    char: '\\',
                    color: ratatui::style::Color::Green,
                },
            ]),
        },
        OsIcon {
            name: "debian",
            logo: [
                "",
                "",
                "            $2_,met$$$$$$$$$$gg.",
                "        ,g$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$P.",
                "    ,g$$$$P\"\"       \"\"\"Y$$$$.\".",
                "    ,$$$$P'              `$$$$$$.    ",
                "    ',$$$$P       ,ggs.     `$$$$b:  ",
                "    `d$$$$'     ,$P\"'   $1.$2    $$$$$$",
                "    $$$$P      d$'     $1,$2    $$$$P ",
                "    $$$$:      $$$.   $1-$2    ,d$$$$'",
                "    $$$$;      Y$b._   _,d$P'",
                "    Y$$$$.    $1`.$2`\"Y$$$$$$$$P\"'",
                "    `$$$$b      $1\"-.__",
                "    $2`Y$$$$b   ",
                "    `Y$$$$.     ",
                "        `$$$$b. ",
                "        `Y$$$$b.",
                "            `\"Y$$b._",
                "                `\"\"\"\"",
                "",
            ],
            color: Vec::from(&[
                Color {
                    char: '$',
                    color: ratatui::style::Color::Red,
                },
                Color {
                    char: '\\',
                    color: ratatui::style::Color::Green,
                },
            ]),
        },
        OsIcon {
            name: "alpine",
            logo: [
                "                       -`",
                "                      .o+`",
                "                     `ooo/",
                "                    `+oooo:",
                "                   `+oooooo:",
                "                   -+oooooo+:",
                "                 `/:-:++oooo+:",
                "                `/++++/+++++++: ",
                "               `/++++++++++++++:",
                "              `/+++o$2oooooooo$1` ",
                "             ./$2ooosssso++osssss`",
                "            .oossssso-````/ossssss+`",
                "           -osssssso.      :ssssssso.",
                "          :osssssss/        osssso+++.",
                "         /ossssssss/        +ssssooo/-",
                "       `/ossssso+/:-        -:/+osssso+-",
                "      `+sso+:-`                 `.-/+oso: ",
                "     `++:.                           `-/+/",
                "     .`                                 `/",
                "                                          ",
            ],
            color: Vec::from(&[]),
        },
        OsIcon {
            name: "arch",
            logo: [
                "                       -`",
                "                      .o+`",
                "                     `ooo/",
                "                    `+oooo: ",
                "                   `+oooooo:",
                "                   -+oooooo+:",
                "                 `/:-:++oooo+:",
                "                `/++++/+++++++:",
                "               `/++++++++++++++:",
                "              `/+++o$2oooooooo$1`",
                "             ./$2ooosssso++osssss`",
                "            .oossssso-````/ossssss+`",
                "           -osssssso.      :ssssssso.",
                "          :osssssss/        osssso+++.",
                "         /ossssssss/        +ssssooo/-",
                "       `/ossssso+/:-        -:/+osssso+-",
                "      `+sso+:-`                 `.-/+oso:",
                "     `++:.                           `-/+/",
                "     .`                                 `/",
                "                                          ",
            ],
            color: Vec::from(&[Color {
                char: 'o',
                color: ratatui::style::Color::Blue,
            }]),
        },
        OsIcon {
            name: "centos",
            logo: [
                "                 .. ",
                "               .PLTJ.",
                "              <><><><>",
                "     $2KKSSV' 4KKK $1LJ$4 KKKL.'VSSKK",
                "     $2KKV' 4KKKKK $1LJ$4 KKKKAL 'VKK",
                "     $2V' ' 'VKKKK $1LJ$4 KKKKV' ' 'V",
                "     $2.4MA.' 'VKK $1LJ$4 KKV' '.4Mb.",
                "   $4. $2KKKKKA.' 'V $1LJ$4 V' '.4KKKKK $3.",
                " $4.4D $2KKKKKKKA.'' $1LJ$4 ''.4KKKKKKK $3FA.",
                "$4<QDD ++++++++++++  $3++++++++++++ GFD>",
                " '$4VD $3KKKKKKKK'.. $2LJ $1..'KKKKKKKK $3FV",
                "   $4' $3VKKKKK'. .4 $2LJ $1K. .'KKKKKV $3'",
                "      $3'VK'. .4KK $2LJ $1KKA. .'KV'",
                "     $3A. . .4KKKK $2LJ $1KKKKA. . .4",
                "     $3KKA. 'KKKKK $2LJ $1KKKKK' .4KK",
                "     $3KKSSA. VKKK $2LJ $1KKKV .4SSKK",
                "              $2<><><><>",
                "               $2'MKKM'",
                "                 $2''",
                "",
            ],
            color: Vec::from(&[]),
        },
        OsIcon {
            name: "fedora",
            logo: [
                "             .',;::::;,'.",
                "         .';:cccccccccccc:;,.",
                "      .;cccccccccccccccccccccc;.",
                "    .:cccccccccccccccccccccccccc:.",
                "  .;ccccccccccccc;$2.:dddl:.$1;ccccccc;.",
                " .:ccccccccccccc;$2OWMKOOXMWd$1;ccccccc:.",
                ".:ccccccccccccc;$2KMMc$1;cc;$2xMMc$1;ccccccc:.",
                ",cccccccccccccc;$2MMM.$1;cc;$2;WW:$1;cccccccc,",
                ":cccccccccccccc;$2MMM.$1;cccccccccccccccc:",
                ":ccccccc;$2oxOOOo$1;$2MMM000k.$1;cccccccccccc:",
                "cccccc;$20MMKxdd:$1;$2MMMkddc.$1;cccccccccccc;",
                "ccccc;$2XMO'$1;cccc;$2MMM.$1;cccccccccccccccc'",
                "ccccc;$2MMo$1;ccccc;$2MMW.$1;ccccccccccccccc;",
                "ccccc;$20MNc.$1ccc$2.xMMd$1;ccccccccccccccc;",
                "cccccc;$2dNMWXXXWM0:$1;cccccccccccccc:,",
                "cccccccc;$2.:odl:.$1;cccccccccccccc:,.",
                "ccccccccccccccccccccccccccccc:'.",
                ":ccccccccccccccccccccccc:;,..",
                " ':cccccccccccccccc::;,.",
                "",
            ],
            color: Vec::from(&[]),
        },
    ]
});

static NORMAL_RENDERED_ICON: LazyLock<RenderedIcon> = LazyLock::new(RenderedIcon::default);
static CURRENT_DISTRO: LazyLock<String> = LazyLock::new(System::distribution_id);
static RENDERED_ICON_MAP: OnceCell<HashMap<String, RenderedIcon>> = OnceCell::new();

fn build_rendered_icon(icon: &OsIcon) -> RenderedIcon {
    let default_color = icon
        .color
        .iter()
        .find(|color| color.char == '\\')
        .map(|color| color.color)
        .unwrap_or(ratatui::style::Color::Reset);

    RenderedIcon {
        lines: icon
            .logo
            .iter()
            .map(|line| {
                line.chars()
                    .map(|char| CachedGlyph {
                        char,
                        color: icon
                            .color
                            .iter()
                            .find(|color| color.char == char)
                            .map(|color| color.color)
                            .unwrap_or(default_color),
                    })
                    .collect()
            })
            .collect(),
    }
}

pub fn init_art() {
    let mut temp = HashMap::new();
    for icon in OS_ICONS.iter() {
        temp.insert(String::from(icon.name), build_rendered_icon(icon));
    }
    RENDERED_ICON_MAP.set(temp).unwrap();
}

#[allow(clippy::cast_possible_truncation)]
pub fn render_logo(area: Rect, buf: &mut Buffer) {
    normal_block("art").render(area, buf);
    let area = area.inner(Margin {
        vertical: 0,
        horizontal: 2,
    });
    for i in 1..=48 {
        let cell = &mut buf[(area.left() + i, area.top() + 1)];
        cell.set_char('-');
        let cell = &mut buf[(area.left() + i, area.top() + 22)];
        cell.set_char('-');
    }
    for i in 2..=21 {
        let cell = &mut buf[(area.left(), area.top() + i)];
        cell.set_char('|');
        let cell = &mut buf[(area.left() + 48, area.top() + i)];
        cell.set_char('|');
    }
    let icon = RENDERED_ICON_MAP
        .get()
        .unwrap()
        .get(CURRENT_DISTRO.as_str())
        .unwrap_or(&NORMAL_RENDERED_ICON);
    for (y, line) in icon.lines.iter().enumerate() {
        for (x, glyph) in line.iter().enumerate() {
            let x = area.left() + x as u16 + 1;
            let y = area.top() + y as u16 + 2;
            let cell = &mut buf[(x, y)];
            cell.set_fg(glyph.color);
            cell.set_char(glyph.char);
        }
    }
}
