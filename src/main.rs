#![deny(unsafe_code)]
use std::{env, fs};
use std::process::Command;
use std::time::Duration;
use serde_derive::{Deserialize, Serialize};
//use tray_icon::{TrayIconBuilder, menu::Menu, menu::MenuItem, menu::PredefinedMenuItem, menu::AboutMetadata, menu::Submenu};
use zbus::{Connection, dbus_interface};
use tray_item::{TrayItem, IconSource};
use std::sync::RwLock;

fn get_home() -> String {
    match env::var("XDG_CONFIG_HOME").or_else(|_| env::var("HOME")) {
        Ok(var) => var,
        Err(..) => panic!("Failed to find config directory, make sure XDG_CONFIG_HOME or HOME are set")
    }
}

#[derive(Serialize, Deserialize)]
struct Config {
    default_offset: Option<i32>,
    interval: Option<u64>,
    divide: Option<i32>,
    sensor: String,
    displays: Vec<Display>,
    minimum: Option<u32>,
    maximum: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone)]
struct Display {
    cmd: String,
}

static OFFSET: RwLock<i32> = RwLock::new(0);

#[derive(Clone)]
struct Autobright;

#[dbus_interface(name = "org.oceania.AutobrightServer")]
impl Autobright {
    fn increase(&mut self, value: i32) -> String {
        let mut offset = OFFSET.write().unwrap();
        *offset += value;
        drop(offset);
        brightness_notify();
        format!("Ok")
    }

    fn decrease(&mut self, value: i32) -> String {
        let mut offset = OFFSET.write().unwrap();
        *offset -= value;
        drop(offset);
        brightness_notify();
        format!("Ok")
    }
}

fn brightness_notify() {
    let offset = OFFSET.read().unwrap();
    Command::new("notify-send").arg("--app-name=Autobrightd").arg(format!("Brightness Offset is: {}", offset).as_str()).spawn();
}



#[tokio::main]
async fn main() {
    let exec = tokio::runtime::Runtime::new().unwrap();
    /*
    exec.spawn(async {
    let tray_menu = Menu::new();
    let submenu = Submenu::new("yes", true);
    let quit_i = MenuItem::new("Quit", true, None);
    submenu.append_items(&[
        &PredefinedMenuItem::about(
            None,
            Some(AboutMetadata {
                name: Some("tao".to_string()),
                copyright: Some("Copyright tao".to_string()),
                ..Default::default()
            }),
        ),
        &PredefinedMenuItem::separator(),
        &quit_i,
    ]);
    tray_menu.append_items(&[&submenu]);
        gtk::init().unwrap();
        let _tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_title("Autobrightd is running!")
            .build()
            .unwrap();
        gtk::main();
        println!("gtk main finished");
    });
    */
    exec.spawn(async {
        gtk::init().unwrap();
        let mut tray = TrayItem::new("Tray Example", IconSource::Resource("preferences-desktop")).unwrap();
        tray.add_menu_item("Increase", || {
            let mut offset = OFFSET.write().unwrap();
            *offset += 5;
            drop(offset);
            brightness_notify();
        }).unwrap();
        tray.add_menu_item("Decrease", || {
            let mut offset = OFFSET.write().unwrap();
            *offset -= 5;
            drop(offset);
            brightness_notify();
        }).unwrap();
        tray.add_menu_item("Quit", || {
            gtk::main_quit();
        }).unwrap();
        gtk::main();
    });

    let cfg_file = fs::read_to_string(format!("{}/Oceania/autobright.toml", get_home())).unwrap();
    let cfg: Config = toml::from_str(cfg_file.as_str()).unwrap();
    let interval = cfg.interval.unwrap_or(5);
    let mut offset = OFFSET.write().unwrap();
    *offset = cfg.default_offset.unwrap_or(0);
    let divide = cfg.divide.unwrap_or(1);
    let minimum = cfg.minimum.unwrap_or(0) as i32;
    let maximum = cfg.maximum.unwrap_or(100) as i32;
    let sensor = cfg.sensor.clone();
    let mut display_out_old = 0;
    let displays = cfg.displays.clone();
    drop(cfg_file);
    drop(cfg);
    drop(offset);

    let connection = Connection::session().await.expect("no message bus");
    match connection.object_server().at("/org/oceania/Autobright", Autobright).await {
        Ok(_) => println!("Successfully made object server"),
        Err(_) => panic!("Failed to create object server")
    }
    match connection.request_name("org.oceania.Autobright").await {
        Ok(_) => println!("Successfully acquired dbus name"),
        Err(_) => panic!("Failed to acquire dbus name, is another autobrightd running?")
    }

    loop {
        let sensor_in: i32 = fs::read_to_string(sensor.clone()).unwrap().trim().parse().unwrap();
        let sensor_in_adjusted = sensor_in / divide;
        let offset = OFFSET.read().unwrap();
        println!("{}", *offset);
        let mut display_out = sensor_in_adjusted + *offset;
        drop(offset);
        if display_out < minimum {
            display_out = minimum;
        } else if display_out > maximum {
            display_out = maximum;
        }
        tokio::time::sleep(Duration::from_millis(interval)).await;
        if display_out_old != display_out {
            for display in &displays {
                Command::new(display.cmd.as_str()).arg(display_out.to_string()).spawn().expect("oops");
            }
        }
        display_out_old = display_out;
    }
}
