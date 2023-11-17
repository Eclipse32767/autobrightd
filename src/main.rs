use std::{env, fs};
use std::process::Command;
use std::time::Duration;
use serde_derive::{Deserialize, Serialize};
use tray_icon::{TrayIconBuilder, menu::Menu};
use zbus::{Connection, dbus_interface};

fn get_home() -> String {
    match env::var("XDG_CONFIG_HOME") {
        Ok(var) => var,
        Err(..) => match env::var("HOME") {
            Ok(var) => format!("{var}/.config"),
            Err(..) => panic!("Failed to find config directory, make sure XDG_CONFIG_HOME or HOME are set")
        }
    }
}
#[derive(Serialize, Deserialize)]
struct Config {
    default_offset: Option<i32>,
    interval: Option<u64>,
    divide: Option<i32>,
    sensor: String,
    displays: Vec<Display>
}
#[derive(Serialize, Deserialize, Clone)]
struct Display {
    cmd: String,
}
static mut OFFSET: i32 = 0;

#[derive(Clone)]
struct Autobright;

#[dbus_interface(name = "org.oceania.AutobrightServer")]
impl Autobright {
    fn increase(&mut self, value: i32) -> String {
        unsafe {
            OFFSET = OFFSET + value;
        }
        format!("Ok")
    }
    fn decrease(&mut self, value: i32) -> String {
        unsafe {
            OFFSET = OFFSET - value;
        }
        format!("Ok")
    }
}




#[tokio::main]
async fn main() {
    let exec = tokio::runtime::Runtime::new().unwrap();

    exec.spawn(async {
        gtk::init().unwrap();
        let tray_menu = Menu::new();
        let _tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .build()
            .unwrap();
        gtk::main();
    });

    let cfg_file = fs::read_to_string(format!("{}/Oceania/autobright.toml", get_home())).unwrap();
    let cfg: Config = toml::from_str(cfg_file.as_str()).unwrap();
    let interval = match cfg.interval {
        Some(value) => value,
        None => 5,
    };
    unsafe {
        OFFSET = match cfg.default_offset {
            Some(value) => value,
            None => 0,
        }
    }
    let divide = match cfg.divide {
        Some(val) => val,
        None => 1,
    };
    let sensor = cfg.sensor.clone();
    let mut display_out_old = 0;
    let displays = cfg.displays.clone();
    drop(cfg_file);
    drop(cfg);

    let connection = Connection::session().await.expect("no message bus");
    connection
        .object_server()
        .at("/org/oceania/Autobright", Autobright)
        .await.expect("no");
    connection
        .request_name("org.oceania.Autobright")
        .await.expect("no");

    loop {
        let offset = unsafe {
            OFFSET
        };
        println!("{}", offset);
        let sensor_in: i32 = fs::read_to_string(sensor.clone()).unwrap().trim().parse().unwrap();
        let sensor_in_adjusted = sensor_in / divide;
        let display_out = sensor_in_adjusted + offset;
        tokio::time::sleep(Duration::from_millis(interval)).await;
        if display_out_old != display_out {
            for display in &displays {
                Command::new(display.cmd.as_str()).arg(display_out.to_string()).spawn().expect("oops");
            }
        }
        display_out_old = display_out;
    }
}
