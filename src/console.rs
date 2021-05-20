use mlua::MultiValue;
use rustyline::Editor;
use std::sync::mpsc::Sender;
use std::io::Write;

pub enum ConsoleMsg {
    Command(String),
    Reload,
    Reset,
    Exit,
}

const PROMPT: &str = "> ";

pub fn console(tx: Sender<ConsoleMsg>) {
    let mut editor = Editor::<()>::new();

    loop {
        match editor.readline(PROMPT) {
            Ok(s) => {
                let cmd = parse_command(s);
                if let ConsoleMsg::Exit = cmd {
                    break;
                }
                tx.send(cmd).expect("Console failed to send message")
            }
            Err(_) => return,
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
            println!(
                "\r{}",
                values
                    .iter()
                    .map(|value| format!("{:?}", value))
                    .collect::<Vec<_>>()
                    .join("\t")
            );
        }
        Err(e) => println!("error: {}", e),
    }
    print!("{}", PROMPT);
    std::io::stdout().lock().flush().unwrap();
}
