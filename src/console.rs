use mlua::MultiValue;
use rustyline::{config, EditMode, Editor};
use std::io::Write;
use std::sync::mpsc::Sender;

pub enum ConsoleMsg {
    Command(String),
    Reload,
    Reset,
    Exit,
}

const PROMPT: &str = "> ";

pub fn console(tx: Sender<ConsoleMsg>) {
    let mut editor =
        Editor::<()>::with_config(config::Builder::new().edit_mode(EditMode::Vi).build());

    loop {
        match editor.readline(PROMPT) {
            Ok(s) => {
                editor.add_history_entry(&s);
                let cmd = parse_command(s);
                if let ConsoleMsg::Exit = cmd {
                    break;
                }
                tx.send(cmd).expect("Console failed to send message")
            }
            Err(_) => break,
        }
    }

    tx.send(ConsoleMsg::Exit)
        .expect("Console failed to send exit")
}

fn parse_command(s: String) -> ConsoleMsg {
    match s.as_str() {
        "/exit" => ConsoleMsg::Exit,
        "/reload" => ConsoleMsg::Reload,
        "/reset" => ConsoleMsg::Reset,
        _ => ConsoleMsg::Command(s),
    }
}

pub fn print_lua_ret(ret: mlua::Result<MultiValue>) {
    match ret {
        Ok(values) => {
            console_print(&format!(
                "\r{}",
                values
                    .iter()
                    .map(|value| format!("{:?}", value))
                    .collect::<Vec<_>>()
                    .join("\t")
            ));
        }
        Err(e) => console_print(&format!("error: {}", e)),
    }
}

pub fn console_print(s: &str) {
    println!("{}", s);
    print!("{}", PROMPT);
    std::io::stdout().lock().flush().unwrap();
}
