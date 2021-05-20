use rustyline::Editor;
use std::sync::mpsc::Sender;

pub enum ConsoleMsg {
    Command(String),
    Reload,
    Reset,
    Exit,
}

pub fn console(tx: Sender<ConsoleMsg>) {
    let mut editor = Editor::<()>::new();
    loop {
        match editor.readline("> ") {
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
}

fn parse_command(s: String) -> ConsoleMsg {
    match s.as_str() {
        "/exit" => ConsoleMsg::Exit,
        "/reload" => ConsoleMsg::Reload,
        "/reset" => ConsoleMsg::Reset,
        _ => ConsoleMsg::Command(s),
    }
}
